use super::novel::ReadingProgress;
use crate::config::CONFIG;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 管理用户的小说库和阅读进度
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Library {
    /// 所有已跟踪的小说信息
    pub novels: Vec<NovelInfo>,
}

/// 小说信息
///
/// 存储小说的标题、路径和阅读进度，用于持久化。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovelInfo {
    pub title: String,
    #[serde(
        serialize_with = "serialize_novel_path",
        deserialize_with = "deserialize_novel_path"
    )]
    pub path: PathBuf,
    pub progress: ReadingProgress,
}

fn serialize_novel_path<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serialized = if let Some(rel) = Library::novel_rel_path(path) {
        let mut normalized = PathBuf::from("novels");
        normalized.push(rel);
        normalized.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().to_string()
    };
    serializer.serialize_str(&serialized)
}

fn deserialize_novel_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path = String::deserialize(deserializer)?;
    Ok(PathBuf::from(path))
}

impl Library {
    /// 创建新的空图书馆
    ///
    /// # Returns
    ///
    /// 一个不包含任何小说的新实例。
    pub fn new() -> Self {
        Library { novels: Vec::new() }
    }

    /// 从文件加载图书馆数据
    ///
    /// 如果进度文件不存在或解析失败，返回一个新的空实例。
    /// 损坏的文件会被备份为 `.json.corrupted.{timestamp}`。
    ///
    /// # Returns
    ///
    /// 加载的图书馆实例，或新实例（如果加载失败）。
    pub fn load() -> Self {
        let progress_path = Self::get_progress_path();
        if progress_path.exists() {
            match std::fs::read_to_string(&progress_path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(mut library) => {
                        let normalized = library.normalize_novel_paths();
                        let reserialized_differs = serde_json::to_string_pretty(&library)
                            .map(|new_content| new_content != content)
                            .unwrap_or(false);
                        if (normalized || reserialized_differs)
                            && let Err(e) = library.save()
                        {
                            eprintln!("Failed to save normalized progress.json: {}", e);
                        }
                        return library;
                    }
                    Err(e) => {
                        eprintln!("Failed to parse progress.json: {}", e);

                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        let corrupted_path =
                            progress_path.with_extension(format!("json.corrupted.{}", timestamp));

                        if let Err(backup_err) = std::fs::copy(&progress_path, &corrupted_path) {
                            eprintln!("Failed to backup corrupted file: {}", backup_err);
                        } else {
                            eprintln!("Corrupted file backed up to: {:?}", corrupted_path);
                        }

                        return Self::new();
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read progress.json: {}", e);
                    return Self::new();
                }
            }
        }
        Self::new()
    }

    /// 保存图书馆数据到文件
    ///
    /// 使用原子写入确保数据完整性，自动创建备份文件。
    ///
    /// # Errors
    ///
    /// 返回 IO 操作或序列化错误。
    pub fn save(&self) -> std::io::Result<()> {
        let progress_path = Self::get_progress_path();
        let content = serde_json::to_string_pretty(self)?;

        let _ = Self::create_backup_if_needed(&progress_path);

        let temp_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_path = progress_path.with_extension(format!("tmp.{}", temp_suffix));
        std::fs::write(&temp_path, &content)?;

        #[cfg(windows)]
        if progress_path.exists() {
            std::fs::remove_file(&progress_path)?;
        }

        std::fs::rename(&temp_path, &progress_path)?;

        Ok(())
    }

    /// 获取进度文件的路径
    ///
    /// # Returns
    ///
    /// 进度文件的完整路径。测试环境下返回临时目录。
    pub fn get_progress_path() -> PathBuf {
        #[cfg(test)]
        {
            let mut path = std::env::temp_dir();
            path.push(format!("{}_test", CONFIG.dir_name));
            let _ = std::fs::create_dir_all(&path);
            path.push(CONFIG.progress_filename);
            return path;
        }

        #[cfg(not(test))]
        {
            let mut path = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(CONFIG.dir_name);

            if !path.exists()
                && let Err(e) = std::fs::create_dir_all(&path)
            {
                eprintln!("Failed to create directory: {}", e);
            }

            path.push(CONFIG.progress_filename);
            path
        }
    }

    fn get_novels_dir() -> PathBuf {
        #[cfg(test)]
        {
            let mut path = std::env::temp_dir();
            path.push(format!("{}_test", CONFIG.dir_name));
            path.push("novels");
            let _ = std::fs::create_dir_all(&path);
            return path;
        }

        #[cfg(not(test))]
        {
            let mut path = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(CONFIG.dir_name);
            path.push("novels");
            if !path.exists()
                && let Err(e) = std::fs::create_dir_all(&path)
            {
                eprintln!("Failed to create novels directory: {}", e);
            }
            path
        }
    }

    fn create_backup_if_needed(progress_path: &Path) -> std::io::Result<()> {
        if !progress_path.exists() {
            return Ok(());
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let period_timestamp =
            timestamp / CONFIG.backup_timestamp_interval * CONFIG.backup_timestamp_interval;

        let file_name = progress_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(CONFIG.progress_filename);
        let backup_name = format!(
            "{}.{}.{}",
            file_name, CONFIG.backup_suffix, period_timestamp
        );
        let backup_path = progress_path.with_file_name(backup_name);

        if backup_path.exists() {
            return Ok(());
        }

        std::fs::copy(progress_path, &backup_path)?;

        let cutoff_timestamp =
            timestamp.saturating_sub(CONFIG.backup_retention_days * 24 * 60 * 60);
        if let Some(backup_dir) = progress_path.parent() {
            Self::cleanup_old_backups(backup_dir, cutoff_timestamp);
        }

        Ok(())
    }

    fn cleanup_old_backups(backup_dir: &Path, cutoff_timestamp: u64) {
        let Ok(entries) = std::fs::read_dir(backup_dir) else {
            return;
        };

        let backup_prefix = format!("{}.{}.", CONFIG.progress_filename, CONFIG.backup_suffix);

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if let Some(ts_str) = name.strip_prefix(&backup_prefix)
                && let Ok(file_timestamp) = ts_str.parse::<u64>()
                && file_timestamp < cutoff_timestamp
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    fn novel_rel_path(path: &Path) -> Option<PathBuf> {
        let raw = path.to_string_lossy();
        let parts: Vec<&str> = raw.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
        let novels_idx = parts
            .iter()
            .rposition(|segment| segment.eq_ignore_ascii_case("novels"))?;
        if novels_idx + 1 >= parts.len() {
            return None;
        }
        let mut rel = PathBuf::new();
        for part in &parts[novels_idx + 1..] {
            rel.push(part);
        }
        Some(rel)
    }

    /// 提取跨平台稳定的小说键（`novels/...`），用于同步后路径匹配。
    fn novel_sync_key(path: &Path) -> Option<String> {
        let rel = Self::novel_rel_path(path)?;
        Some(format!(
            "novels/{}",
            rel.to_string_lossy().replace('\\', "/")
        ))
    }

    fn normalize_novel_paths(&mut self) -> bool {
        let novels_dir = Self::get_novels_dir();
        let mut changed = false;
        for novel in &mut self.novels {
            if let Some(rel) = Self::novel_rel_path(&novel.path) {
                let normalized = novels_dir.join(rel);
                if novel.path != normalized {
                    novel.path = normalized;
                    changed = true;
                }
            }
        }
        changed
    }

    fn same_novel_path(a: &Path, b: &Path) -> bool {
        if a == b {
            return true;
        }
        match (Self::novel_sync_key(a), Self::novel_sync_key(b)) {
            (Some(a_key), Some(b_key)) => a_key == b_key,
            _ => false,
        }
    }

    /// 更新或添加小说的阅读进度
    ///
    /// 如果小说已存在则更新进度，否则创建新条目。
    ///
    /// # Arguments
    ///
    /// * `novel_path` - 小说文件路径
    /// * `progress` - 阅读进度
    pub fn update_novel_progress(&mut self, novel_path: &Path, progress: ReadingProgress) {
        if let Some(novel) = self
            .novels
            .iter_mut()
            .find(|n| Self::same_novel_path(&n.path, novel_path))
        {
            novel.progress = progress;
            novel.path = novel_path.to_path_buf();
        } else {
            let title = novel_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("未知标题")
                .to_string();

            self.novels.push(NovelInfo {
                title,
                path: novel_path.to_path_buf(),
                progress,
            });
        }
    }

    /// 获取小说的阅读进度
    ///
    /// # Arguments
    ///
    /// * `novel_path` - 小说文件路径
    ///
    /// # Returns
    ///
    /// 小说的阅读进度，如果小说不存在则返回默认进度。
    pub fn get_novel_progress(&self, novel_path: &Path) -> ReadingProgress {
        self.novels
            .iter()
            .find(|n| Self::same_novel_path(&n.path, novel_path))
            .map(|n| n.progress.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn progress_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clean_progress_artifacts(progress_path: &Path) {
        let _ = std::fs::remove_file(progress_path);
        if let Some(parent) = progress_path.parent()
            && let Ok(entries) = std::fs::read_dir(parent)
        {
            let prefix = format!("{}.", CONFIG.progress_filename);
            for entry in entries.flatten() {
                let p = entry.path();
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with(&prefix) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }

    #[test]
    fn test_update_and_get_progress() {
        let mut library = Library::new();
        let novel_path = PathBuf::from("/path/to/novel.txt");
        let progress = ReadingProgress {
            scroll_offset: 100,
            bookmarks: Vec::new(),
        };

        library.update_novel_progress(&novel_path, progress.clone());
        assert_eq!(library.get_novel_progress(&novel_path), progress);
        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.novels[0].title, "novel");

        let new_progress = ReadingProgress {
            scroll_offset: 200,
            bookmarks: Vec::new(),
        };
        library.update_novel_progress(&novel_path, new_progress.clone());
        assert_eq!(library.get_novel_progress(&novel_path), new_progress);
        assert_eq!(library.novels.len(), 1);
    }

    #[test]
    fn test_library_new() {
        let library = Library::new();
        assert!(library.novels.is_empty());
    }

    #[test]
    fn test_update_novel_progress_creates_new_entry() {
        let mut library = Library::new();
        let path = PathBuf::from("/test/novel.txt");
        let progress = ReadingProgress {
            scroll_offset: 50,
            bookmarks: Vec::new(),
        };

        library.update_novel_progress(&path, progress.clone());

        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.novels[0].title, "novel");
        assert_eq!(library.novels[0].progress, progress);
    }

    #[test]
    fn test_get_novel_progress_not_found() {
        let library = Library::new();
        let path = PathBuf::from("/nonexistent/novel.txt");

        let progress = library.get_novel_progress(&path);

        assert_eq!(progress, ReadingProgress::default());
    }

    #[test]
    fn test_get_novel_progress_matches_cross_platform_paths() {
        let mut library = Library::new();
        library.novels.push(NovelInfo {
            title: "demo".to_string(),
            path: PathBuf::from(r"C:\Users\alice\.fish_reader\novels\demo.txt"),
            progress: ReadingProgress {
                scroll_offset: 123,
                bookmarks: Vec::new(),
            },
        });

        let progress =
            library.get_novel_progress(Path::new("/Users/alice/.fish_reader/novels/demo.txt"));
        assert_eq!(progress.scroll_offset, 123);
    }

    #[test]
    fn test_update_novel_progress_migrates_path_when_sync_key_matches() {
        let mut library = Library::new();
        library.novels.push(NovelInfo {
            title: "demo".to_string(),
            path: PathBuf::from(r"C:\Users\alice\.fish_reader\novels\demo.txt"),
            progress: ReadingProgress {
                scroll_offset: 10,
                bookmarks: Vec::new(),
            },
        });

        let local_path = PathBuf::from("/Users/alice/.fish_reader/novels/demo.txt");
        let new_progress = ReadingProgress {
            scroll_offset: 456,
            bookmarks: Vec::new(),
        };
        library.update_novel_progress(&local_path, new_progress.clone());

        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.novels[0].path, local_path);
        assert_eq!(library.novels[0].progress, new_progress);
    }

    #[test]
    fn test_load_normalizes_cross_platform_path_to_local_novels_dir() {
        let _guard = progress_test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let progress_path = Library::get_progress_path();
        clean_progress_artifacts(&progress_path);

        let content = serde_json::json!({
            "novels": [
                {
                    "title": "demo",
                    "path": r"C:\Users\alice\.fish_reader\novels\demo.txt",
                    "progress": { "scroll_offset": 88, "bookmarks": [] }
                }
            ]
        });
        std::fs::write(
            &progress_path,
            serde_json::to_string_pretty(&content).unwrap(),
        )
        .unwrap();

        let loaded = Library::load();
        assert_eq!(loaded.novels.len(), 1);
        let expected = Library::get_novels_dir().join("demo.txt");
        assert_eq!(loaded.novels[0].path, expected);
        assert_eq!(loaded.novels[0].progress.scroll_offset, 88);

        let persisted_content = std::fs::read_to_string(&progress_path).unwrap();
        let persisted: serde_json::Value = serde_json::from_str(&persisted_content).unwrap();
        assert_eq!(
            persisted["novels"][0]["path"].as_str().unwrap(),
            "novels/demo.txt"
        );

        clean_progress_artifacts(&progress_path);
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let _guard = progress_test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let progress_path = Library::get_progress_path();
        clean_progress_artifacts(&progress_path);

        let mut library = Library::new();
        let novel_path = PathBuf::from("/tmp/round_trip.txt");
        library.update_novel_progress(
            &novel_path,
            ReadingProgress {
                scroll_offset: 42,
                bookmarks: Vec::new(),
            },
        );
        library.save().unwrap();

        let loaded = Library::load();
        assert_eq!(loaded.novels.len(), 1);
        assert_eq!(loaded.novels[0].path, novel_path);
        assert_eq!(loaded.novels[0].progress.scroll_offset, 42);

        clean_progress_artifacts(&progress_path);
    }

    #[test]
    fn test_load_corrupted_file_returns_new_and_creates_backup() {
        let _guard = progress_test_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let progress_path = Library::get_progress_path();
        clean_progress_artifacts(&progress_path);
        std::fs::write(&progress_path, "{ this is not valid json").unwrap();

        let loaded = Library::load();
        assert!(loaded.novels.is_empty());

        let mut has_corrupted_backup = false;
        if let Some(parent) = progress_path.parent()
            && let Ok(entries) = std::fs::read_dir(parent)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.contains(".json.corrupted.") {
                    has_corrupted_backup = true;
                    break;
                }
            }
        }
        assert!(has_corrupted_backup);

        clean_progress_artifacts(&progress_path);
    }
}
