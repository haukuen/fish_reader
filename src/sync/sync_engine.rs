use crate::sync::config::WebDavConfig;
use crate::sync::webdav_client::WebDavClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::Sender;

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

enum DiffAction {
    Upload(String),
    Delete(String),
    Download(String),
}

fn diff_for_upload(
    local: &HashMap<String, FileEntry>,
    remote: &HashMap<String, FileEntry>,
) -> Vec<DiffAction> {
    let mut actions = Vec::new();

    for (path, local_entry) in local {
        match remote.get(path) {
            Some(remote_entry) if remote_entry.hash == local_entry.hash => {}
            _ => actions.push(DiffAction::Upload(path.clone())),
        }
    }

    for path in remote.keys() {
        if !local.contains_key(path) {
            actions.push(DiffAction::Delete(path.clone()));
        }
    }

    actions
}

fn diff_for_download(
    local: &HashMap<String, FileEntry>,
    remote: &HashMap<String, FileEntry>,
) -> Vec<DiffAction> {
    let mut actions = Vec::new();

    for (path, remote_entry) in remote {
        match local.get(path) {
            Some(local_entry) if local_entry.hash == remote_entry.hash => {}
            _ => actions.push(DiffAction::Download(path.clone())),
        }
    }

    for path in local.keys() {
        if !remote.contains_key(path) {
            actions.push(DiffAction::Delete(path.clone()));
        }
    }

    actions
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

    fn data_dir() -> PathBuf {
        home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".fish_reader")
    }

    fn manifest_local_path() -> PathBuf {
        Self::data_dir().join("sync_manifest.json")
    }

    fn remote_base(&self) -> String {
        self.config.remote_path.trim_end_matches('/').to_string()
    }

    fn remote_file_path(&self, filename: &str) -> String {
        format!("{}/{}", self.remote_base(), filename)
    }

    /// 校验 rel_path 不包含路径穿越，返回安全的本地路径
    fn safe_local_path(data_dir: &Path, rel_path: &str) -> anyhow::Result<PathBuf> {
        let rel = Path::new(rel_path);
        for component in rel.components() {
            match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    anyhow::bail!("不安全的路径: {}", rel_path);
                }
                _ => {}
            }
        }
        let full = data_dir.join(rel);
        Ok(full)
    }

    fn load_local_manifest() -> SyncManifest {
        let path = Self::manifest_local_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(manifest) = serde_json::from_str(&content)
        {
            return manifest;
        }
        SyncManifest::new()
    }

    fn save_local_manifest(manifest: &SyncManifest) -> anyhow::Result<()> {
        let path = Self::manifest_local_path();
        let content = serde_json::to_string_pretty(manifest)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn download_remote_manifest(&self) -> anyhow::Result<Option<SyncManifest>> {
        let remote_path = self.remote_file_path("manifest.json");
        match self.client.download_bytes_opt(&remote_path)? {
            Some(bytes) => {
                let manifest: SyncManifest = serde_json::from_slice(&bytes)?;
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }

    fn upload_manifest(&self, manifest: &SyncManifest) -> anyhow::Result<()> {
        let remote_path = self.remote_file_path("manifest.json");
        let data = serde_json::to_string_pretty(manifest)?;
        self.client.upload_bytes(data.as_bytes(), &remote_path)
    }

    /// 扫描本地文件，构建清单。mtime 未变时复用旧哈希避免读取大文件。
    fn scan_local_files(old_manifest: &SyncManifest) -> anyhow::Result<HashMap<String, FileEntry>> {
        let data_dir = Self::data_dir();
        let mut files = HashMap::new();

        let novels_dir = data_dir.join("novels");
        if novels_dir.exists() {
            for entry in walkdir::WalkDir::new(&novels_dir) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                    let relative = path.strip_prefix(&data_dir)?;
                    let key = relative.to_string_lossy().replace('\\', "/");
                    let meta = std::fs::metadata(path)?;
                    let mtime = meta
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs();
                    let size = meta.len();

                    if let Some(old) = old_manifest.files.get(&key)
                        && old.mtime == mtime
                        && old.size == size
                    {
                        files.insert(key, old.clone());
                        continue;
                    }

                    let contents = std::fs::read(path)?;
                    let hash = crc32fast::hash(&contents);
                    files.insert(key, FileEntry { hash, size, mtime });
                }
            }
        }

        let progress_path = data_dir.join("progress.json");
        if progress_path.exists() {
            let meta = std::fs::metadata(&progress_path)?;
            let mtime = meta
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let size = meta.len();

            let key = "progress.json".to_string();
            if let Some(old) = old_manifest.files.get(&key) {
                if old.mtime == mtime && old.size == size {
                    files.insert(key, old.clone());
                } else {
                    let contents = std::fs::read(&progress_path)?;
                    let hash = crc32fast::hash(&contents);
                    files.insert(key, FileEntry { hash, size, mtime });
                }
            } else {
                let contents = std::fs::read(&progress_path)?;
                let hash = crc32fast::hash(&contents);
                files.insert(key, FileEntry { hash, size, mtime });
            }
        }

        Ok(files)
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

        let total = actions.len();
        for (i, action) in actions.iter().enumerate() {
            match action {
                DiffAction::Upload(rel_path) => {
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

    /// 合并远程 progress.json 与本地：取较大的 scroll_offset，书签取并集
    fn merge_progress(data_dir: &Path, remote_bytes: &[u8]) -> anyhow::Result<()> {
        let progress_path = data_dir.join("progress.json");

        let remote: serde_json::Value = serde_json::from_slice(remote_bytes)?;

        if !progress_path.exists() {
            std::fs::write(&progress_path, remote_bytes)?;
            return Ok(());
        }

        let local_content = std::fs::read_to_string(&progress_path)?;
        let local: serde_json::Value = serde_json::from_str(&local_content)?;

        let merged = Self::merge_library_json(&local, &remote);
        let output = serde_json::to_string_pretty(&merged)?;
        std::fs::write(&progress_path, output)?;

        Ok(())
    }

    /// 按小说合并 Library JSON：取较大 scroll_offset，书签取并集
    fn merge_library_json(
        local: &serde_json::Value,
        remote: &serde_json::Value,
    ) -> serde_json::Value {
        let empty_arr = serde_json::Value::Array(vec![]);

        let local_novels = local
            .get("novels")
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();
        let remote_novels = remote
            .get("novels")
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut local_map: HashMap<String, serde_json::Value> = HashMap::new();
        for novel in &local_novels {
            if let Some(title) = novel.get("title").and_then(|t| t.as_str()) {
                local_map.insert(title.to_string(), novel.clone());
            }
        }

        let mut merged_novels: Vec<serde_json::Value> = Vec::new();
        let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();

        for remote_novel in &remote_novels {
            let title = remote_novel
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            seen_titles.insert(title.clone());

            if let Some(local_novel) = local_map.get(&title) {
                merged_novels.push(Self::merge_novel(local_novel, remote_novel));
            } else {
                merged_novels.push(remote_novel.clone());
            }
        }

        for local_novel in &local_novels {
            let title = local_novel
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            if !seen_titles.contains(&title) {
                merged_novels.push(local_novel.clone());
            }
        }

        for novel in &mut merged_novels {
            Self::normalize_novel_json_path(novel);
        }

        serde_json::json!({ "novels": merged_novels })
    }

    fn novels_rel_path(path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
        let novels_idx = parts
            .iter()
            .rposition(|segment| segment.eq_ignore_ascii_case("novels"))?;
        if novels_idx + 1 >= parts.len() {
            return None;
        }
        Some(parts[novels_idx + 1..].join("/"))
    }

    fn normalize_novel_json_path(novel: &mut serde_json::Value) {
        let Some(path_str) = novel.get("path").and_then(|p| p.as_str()) else {
            return;
        };
        if let Some(rel) = Self::novels_rel_path(path_str) {
            novel["path"] = serde_json::json!(format!("novels/{}", rel));
        }
    }

    fn merge_novel(local: &serde_json::Value, remote: &serde_json::Value) -> serde_json::Value {
        let mut merged = remote.clone();
        if let Some(local_path) = local.get("path") {
            merged["path"] = local_path.clone();
        }

        let local_offset = local
            .get("progress")
            .and_then(|p| p.get("scroll_offset"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let remote_offset = remote
            .get("progress")
            .and_then(|p| p.get("scroll_offset"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let max_offset = local_offset.max(remote_offset);

        let empty_arr = serde_json::Value::Array(vec![]);
        let local_bookmarks = local
            .get("progress")
            .and_then(|p| p.get("bookmarks"))
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();
        let remote_bookmarks = remote
            .get("progress")
            .and_then(|p| p.get("bookmarks"))
            .unwrap_or(&empty_arr)
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut seen_positions: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut merged_bookmarks: Vec<serde_json::Value> = Vec::new();

        for bm in remote_bookmarks.iter().chain(local_bookmarks.iter()) {
            let pos = bm.get("position").and_then(|p| p.as_u64()).unwrap_or(0);
            if seen_positions.insert(pos) {
                merged_bookmarks.push(bm.clone());
            }
        }
        merged_bookmarks.sort_by_key(|bm| bm.get("position").and_then(|p| p.as_u64()).unwrap_or(0));

        if let Some(progress) = merged.get_mut("progress") {
            progress["scroll_offset"] = serde_json::json!(max_offset);
            progress["bookmarks"] = serde_json::json!(merged_bookmarks);
        }

        merged
    }
}

#[cfg(test)]
mod tests {
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
