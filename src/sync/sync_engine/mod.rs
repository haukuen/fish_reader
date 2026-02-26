use crate::sync::config::WebDavConfig;
use crate::sync::webdav_client::WebDavClient;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::mpsc::Sender;

mod diff;
mod io;
mod merge;

use diff::{DiffAction, diff_for_download, diff_for_upload};

/// 同步进度消息
pub enum SyncMessage {
    /// 进度更新（显示在状态栏）
    Progress(String),
    /// 上传完成
    UploadComplete,
    /// 下载完成（需要重新加载数据）
    DownloadComplete,
    /// 操作失败
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub version: u32,
    pub last_sync: u64,
    pub files: HashMap<String, FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub hash: u32,
    pub size: u64,
    pub mtime: u64,
}

impl SyncManifest {
    fn new() -> Self {
        Self {
            version: 1,
            last_sync: 0,
            files: HashMap::new(),
        }
    }
}

pub struct SyncEngine {
    client: WebDavClient,
    config: WebDavConfig,
}

impl SyncEngine {
    pub fn new(config: &WebDavConfig) -> anyhow::Result<Self> {
        let client = WebDavClient::new(config)?;
        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// 上传同步（后台线程调用）
    pub fn sync_up(&self, tx: &Sender<SyncMessage>) {
        if let Err(e) = self.do_sync_up(tx) {
            tx.send(SyncMessage::Failed(e.to_string())).ok();
        }
    }

    /// 下载同步（后台线程调用）
    pub fn sync_down(&self, tx: &Sender<SyncMessage>) {
        if let Err(e) = self.do_sync_down(tx) {
            tx.send(SyncMessage::Failed(e.to_string())).ok();
        }
    }

    fn do_sync_up(&self, tx: &Sender<SyncMessage>) -> anyhow::Result<()> {
        let data_dir = Self::data_dir();

        tx.send(SyncMessage::Progress("扫描本地文件...".into()))
            .ok();
        let old_manifest = Self::load_local_manifest();
        let local_files = Self::scan_local_files(&old_manifest)?;

        let remote_manifest = self
            .download_remote_manifest()?
            .unwrap_or_else(SyncManifest::new);

        let actions = diff_for_upload(&local_files, &remote_manifest.files);
        if actions.is_empty() {
            tx.send(SyncMessage::Progress("没有需要同步的变更".into()))
                .ok();
            tx.send(SyncMessage::UploadComplete).ok();
            return Ok(());
        }

        let base = self.remote_base();
        self.client.mkcol(&format!("{}/", base))?;
        self.client.mkcol(&format!("{}/novels/", base))?;

        // 收集所有需要创建的远程父目录，避免嵌套路径上传失败
        let mut created_dirs: HashSet<String> = HashSet::new();

        let total = actions.len();
        for (i, action) in actions.iter().enumerate() {
            match action {
                DiffAction::Upload(rel_path) => {
                    // 确保远程父目录存在
                    if let Some(parent) = Path::new(rel_path).parent() {
                        let parent_str = parent.to_string_lossy().replace('\\', "/");
                        if !parent_str.is_empty() && created_dirs.insert(parent_str.clone()) {
                            // 逐级创建父目录
                            let mut cumulative = String::new();
                            for segment in parent_str.split('/') {
                                if cumulative.is_empty() {
                                    cumulative = segment.to_string();
                                } else {
                                    cumulative = format!("{}/{}", cumulative, segment);
                                }
                                let remote_dir = format!("{}/{}/", base, cumulative);
                                self.client.mkcol(&remote_dir)?;
                            }
                        }
                    }

                    let display_name = Path::new(rel_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    tx.send(SyncMessage::Progress(format!(
                        "上传 ({}/{}) {}...",
                        i + 1,
                        total,
                        display_name
                    )))
                    .ok();
                    let local_path = data_dir.join(rel_path);
                    let contents = std::fs::read(&local_path)?;
                    let remote_path = self.remote_file_path(rel_path);
                    self.client.upload_bytes(&contents, &remote_path)?;
                }
                DiffAction::Delete(rel_path) => {
                    let display_name = Path::new(rel_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    tx.send(SyncMessage::Progress(format!(
                        "删除 ({}/{}) {}...",
                        i + 1,
                        total,
                        display_name
                    )))
                    .ok();
                    let remote_path = self.remote_file_path(rel_path);
                    self.client.delete(&remote_path)?;
                }
                DiffAction::Download(_) => {}
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let new_manifest = SyncManifest {
            version: 1,
            last_sync: now,
            files: local_files,
        };
        self.upload_manifest(&new_manifest)?;
        Self::save_local_manifest(&new_manifest)?;

        tx.send(SyncMessage::UploadComplete).ok();
        Ok(())
    }

    fn do_sync_down(&self, tx: &Sender<SyncMessage>) -> anyhow::Result<()> {
        let data_dir = Self::data_dir();

        tx.send(SyncMessage::Progress("获取远程清单...".into()))
            .ok();
        let remote_manifest = self
            .download_remote_manifest()?
            .ok_or_else(|| anyhow::anyhow!("远程没有同步数据"))?;

        let old_manifest = Self::load_local_manifest();
        let local_files = Self::scan_local_files(&old_manifest)?;
        let actions = diff_for_download(&local_files, &remote_manifest.files);

        if actions.is_empty() {
            tx.send(SyncMessage::Progress("没有需要同步的变更".into()))
                .ok();
            tx.send(SyncMessage::DownloadComplete).ok();
            return Ok(());
        }

        let total = actions.len();
        let mut downloaded_progress = false;
        for (i, action) in actions.iter().enumerate() {
            match action {
                DiffAction::Download(rel_path) => {
                    let display_name = Path::new(rel_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    tx.send(SyncMessage::Progress(format!(
                        "下载 ({}/{}) {}...",
                        i + 1,
                        total,
                        display_name
                    )))
                    .ok();

                    let remote_path = self.remote_file_path(rel_path);
                    let bytes = self.client.download_bytes(&remote_path)?;

                    if rel_path == "progress.json" {
                        Self::merge_progress(&data_dir, &bytes)?;
                        downloaded_progress = true;
                    } else {
                        let local_path = Self::safe_local_path(&data_dir, rel_path)?;
                        if let Some(parent) = local_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(&local_path, &bytes)?;
                    }
                }
                DiffAction::Delete(rel_path) => {
                    let display_name = Path::new(rel_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    tx.send(SyncMessage::Progress(format!(
                        "删除 ({}/{}) {}...",
                        i + 1,
                        total,
                        display_name
                    )))
                    .ok();
                    let local_path = Self::safe_local_path(&data_dir, rel_path)?;
                    std::fs::remove_file(&local_path).ok();
                }
                DiffAction::Upload(_) => {}
            }
        }

        let mut final_manifest = remote_manifest;
        if downloaded_progress {
            let progress_path = data_dir.join("progress.json");
            if progress_path.exists() {
                let contents = std::fs::read(&progress_path)?;
                let meta = std::fs::metadata(&progress_path)?;
                let mtime = meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                final_manifest.files.insert(
                    "progress.json".to_string(),
                    FileEntry {
                        hash: crc32fast::hash(&contents),
                        size: meta.len(),
                        mtime,
                    },
                );
            }
        }
        Self::save_local_manifest(&final_manifest)?;

        tx.send(SyncMessage::DownloadComplete).ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::diff::{DiffAction, diff_for_download, diff_for_upload};
    use super::*;

    fn entry(hash: u32) -> FileEntry {
        FileEntry {
            hash,
            size: 1,
            mtime: 1,
        }
    }

    #[test]
    fn test_diff_for_upload_detects_upload_and_delete() {
        let mut local = HashMap::new();
        local.insert("novels/same.txt".to_string(), entry(10));
        local.insert("novels/changed.txt".to_string(), entry(20));
        local.insert("progress.json".to_string(), entry(30));

        let mut remote = HashMap::new();
        remote.insert("novels/same.txt".to_string(), entry(10));
        remote.insert("novels/changed.txt".to_string(), entry(99));
        remote.insert("novels/removed.txt".to_string(), entry(40));

        let actions = diff_for_upload(&local, &remote);
        let mut uploads = Vec::new();
        let mut deletes = Vec::new();

        for action in actions {
            match action {
                DiffAction::Upload(path) => uploads.push(path),
                DiffAction::Delete(path) => deletes.push(path),
                DiffAction::Download(_) => panic!("unexpected download action"),
            }
        }

        uploads.sort();
        deletes.sort();
        assert_eq!(
            uploads,
            vec![
                "novels/changed.txt".to_string(),
                "progress.json".to_string()
            ]
        );
        assert_eq!(deletes, vec!["novels/removed.txt".to_string()]);
    }

    #[test]
    fn test_diff_for_download_detects_download_and_delete() {
        let mut local = HashMap::new();
        local.insert("novels/same.txt".to_string(), entry(10));
        local.insert("novels/changed.txt".to_string(), entry(20));
        local.insert("novels/local_only.txt".to_string(), entry(30));

        let mut remote = HashMap::new();
        remote.insert("novels/same.txt".to_string(), entry(10));
        remote.insert("novels/changed.txt".to_string(), entry(99));
        remote.insert("progress.json".to_string(), entry(40));

        let actions = diff_for_download(&local, &remote);
        let mut downloads = Vec::new();
        let mut deletes = Vec::new();

        for action in actions {
            match action {
                DiffAction::Download(path) => downloads.push(path),
                DiffAction::Delete(path) => deletes.push(path),
                DiffAction::Upload(_) => panic!("unexpected upload action"),
            }
        }

        downloads.sort();
        deletes.sort();
        assert_eq!(
            downloads,
            vec![
                "novels/changed.txt".to_string(),
                "progress.json".to_string()
            ]
        );
        assert_eq!(deletes, vec!["novels/local_only.txt".to_string()]);
    }

    #[test]
    fn test_merge_novel_uses_max_offset_and_dedup_bookmarks() {
        let local = serde_json::json!({
            "title": "A",
            "path": "/local/.fish_reader/novels/A.txt",
            "progress": {
                "scroll_offset": 200,
                "bookmarks": [
                    {"name": "l10", "position": 10, "timestamp": 1},
                    {"name": "l20", "position": 20, "timestamp": 2}
                ]
            }
        });
        let remote = serde_json::json!({
            "title": "A",
            "path": "/remote/.fish_reader/novels/A.txt",
            "progress": {
                "scroll_offset": 100,
                "bookmarks": [
                    {"name": "r10", "position": 10, "timestamp": 9},
                    {"name": "r30", "position": 30, "timestamp": 3}
                ]
            }
        });

        let merged = SyncEngine::merge_novel(&local, &remote);
        let mut normalized = merged.clone();
        SyncEngine::normalize_novel_json_path(&mut normalized);
        assert_eq!(normalized["path"].as_str().unwrap(), "novels/A.txt");
        assert_eq!(merged["progress"]["scroll_offset"].as_u64().unwrap(), 200);

        let bookmarks = merged["progress"]["bookmarks"].as_array().unwrap();
        let positions: Vec<u64> = bookmarks
            .iter()
            .map(|b| b["position"].as_u64().unwrap())
            .collect();
        assert_eq!(positions, vec![10, 20, 30]);
        assert_eq!(bookmarks[0]["name"].as_str().unwrap(), "r10");
    }

    #[test]
    fn test_merge_library_json_merges_common_and_keeps_unique() {
        let local = serde_json::json!({
            "novels": [
                {
                    "title": "A",
                    "path": "/local/.fish_reader/novels/A.txt",
                    "progress": {"scroll_offset": 8, "bookmarks": []}
                },
                {
                    "title": "L-only",
                    "path": "/local/.fish_reader/novels/L-only.txt",
                    "progress": {"scroll_offset": 1, "bookmarks": []}
                }
            ]
        });
        let remote = serde_json::json!({
            "novels": [
                {
                    "title": "A",
                    "path": "/remote/.fish_reader/novels/A.txt",
                    "progress": {"scroll_offset": 5, "bookmarks": []}
                },
                {
                    "title": "R-only",
                    "path": "/remote/.fish_reader/novels/R-only.txt",
                    "progress": {"scroll_offset": 2, "bookmarks": []}
                }
            ]
        });

        let merged = SyncEngine::merge_library_json(&local, &remote);
        let novels = merged["novels"].as_array().unwrap();
        assert_eq!(novels.len(), 3);

        let a = novels
            .iter()
            .find(|n| n["title"].as_str() == Some("A"))
            .unwrap();
        assert_eq!(a["progress"]["scroll_offset"].as_u64().unwrap(), 8);
        assert_eq!(a["path"].as_str().unwrap(), "novels/A.txt");
        let l_only = novels
            .iter()
            .find(|n| n["title"].as_str() == Some("L-only"))
            .unwrap();
        assert_eq!(l_only["path"].as_str().unwrap(), "novels/L-only.txt");
        let r_only = novels
            .iter()
            .find(|n| n["title"].as_str() == Some("R-only"))
            .unwrap();
        assert_eq!(r_only["path"].as_str().unwrap(), "novels/R-only.txt");
        assert!(novels.iter().any(|n| n["title"].as_str() == Some("L-only")));
        assert!(novels.iter().any(|n| n["title"].as_str() == Some("R-only")));
    }
}
