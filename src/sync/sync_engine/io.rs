use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
use crate::config::CONFIG;

use super::{FileEntry, SyncEngine, SyncManifest};

impl SyncEngine {
    #[cfg(test)]
    fn test_data_dir() -> PathBuf {
        let mut path = std::env::temp_dir();
        let thread_id = format!("{:?}", std::thread::current().id())
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        path.push(format!(
            "{}_test_{}_{}",
            CONFIG.dir_name,
            std::process::id(),
            thread_id
        ));
        path
    }

    pub(super) fn data_dir() -> PathBuf {
        #[cfg(test)]
        {
            let path = Self::test_data_dir();
            let _ = std::fs::create_dir_all(&path);
            return path;
        }

        #[cfg(not(test))]
        {
            home::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".fish_reader")
        }
    }

    pub(super) fn manifest_local_path() -> PathBuf {
        Self::data_dir().join("sync_manifest.json")
    }

    pub(super) fn remote_base(&self) -> String {
        self.config.remote_path.trim_end_matches('/').to_string()
    }

    pub(super) fn remote_file_path(&self, filename: &str) -> String {
        format!("{}/{}", self.remote_base(), filename)
    }

    /// 校验 rel_path 不包含路径穿越，返回安全的本地路径
    pub(super) fn safe_local_path(data_dir: &Path, rel_path: &str) -> anyhow::Result<PathBuf> {
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

    pub(super) fn load_local_manifest() -> SyncManifest {
        let path = Self::manifest_local_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(manifest) = serde_json::from_str(&content)
        {
            return manifest;
        }
        SyncManifest::new()
    }

    pub(super) fn save_local_manifest(manifest: &SyncManifest) -> anyhow::Result<()> {
        let path = Self::manifest_local_path();
        let content = serde_json::to_string_pretty(manifest)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub(super) fn download_remote_manifest(&self) -> anyhow::Result<Option<SyncManifest>> {
        let remote_path = self.remote_file_path("manifest.json");
        match self.client.download_bytes_opt(&remote_path)? {
            Some(bytes) => {
                let manifest: SyncManifest = serde_json::from_slice(&bytes)?;
                Ok(Some(manifest))
            }
            None => Ok(None),
        }
    }

    pub(super) fn upload_manifest(&self, manifest: &SyncManifest) -> anyhow::Result<()> {
        let remote_path = self.remote_file_path("manifest.json");
        let data = serde_json::to_string_pretty(manifest)?;
        self.client.upload_bytes(data.as_bytes(), &remote_path)
    }

    /// 扫描本地文件，构建清单。mtime 未变时复用旧哈希避免读取大文件。
    pub(super) fn scan_local_files(
        old_manifest: &SyncManifest,
    ) -> anyhow::Result<HashMap<String, FileEntry>> {
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
}
