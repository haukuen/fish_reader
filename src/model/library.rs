use super::novel::ReadingProgress;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 管理用户的小说库和阅读进度
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Library {
    /// 所有已跟踪的小说信息
    pub novels: Vec<NovelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NovelInfo {
    pub title: String,
    pub path: PathBuf,
    pub progress: ReadingProgress,
}

impl Library {
    pub fn new() -> Self {
        Library { novels: Vec::new() }
    }

    pub fn load() -> Self {
        let progress_path = Self::get_progress_path();
        if progress_path.exists() {
            match std::fs::read_to_string(&progress_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(library) => return library,
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

    /// 持久化保存进度数据
    /// # 错误
    /// 返回IO操作或序列化错误
    pub fn save(&self) -> std::io::Result<()> {
        let progress_path = Self::get_progress_path();
        let content = serde_json::to_string_pretty(self)?;

        // 限制备份频率，忽略备份错误（不影响主流程）
        let _ = Self::create_backup_if_needed(&progress_path);

        // 原子写入：先写临时文件，再重命名
        let temp_path = progress_path.with_extension("tmp");
        std::fs::write(&temp_path, &content)?;

        #[cfg(windows)]
        if progress_path.exists() {
            std::fs::remove_file(&progress_path)?;
        }

        std::fs::rename(&temp_path, &progress_path)?;

        Ok(())
    }

    pub fn get_progress_path() -> PathBuf {
        #[cfg(test)]
        {
            let mut path = std::env::temp_dir();
            path.push("fish_reader_test");
            let _ = std::fs::create_dir_all(&path);
            path.push("progress.json");
            return path;
        }

        #[cfg(not(test))]
        {
            let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(".fish_reader");

            if !path.exists()
                && let Err(e) = std::fs::create_dir_all(&path)
            {
                eprintln!("Failed to create directory: {}", e);
            }

            path.push("progress.json");
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

        let period_timestamp = timestamp / 600 * 600;
        let backup_path = progress_path.with_extension(format!("json.backup.{}", period_timestamp));

        if backup_path.exists() {
            return Ok(());
        }

        std::fs::copy(progress_path, &backup_path)?;

        // 清理 3 天前的备份
        let three_days_ago = timestamp.saturating_sub(3 * 24 * 60 * 60);
        if let Some(backup_dir) = progress_path.parent() {
            Self::cleanup_old_backups(backup_dir, three_days_ago);
        }

        Ok(())
    }

    fn cleanup_old_backups(backup_dir: &Path, cutoff_timestamp: u64) {
        let Ok(entries) = std::fs::read_dir(backup_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // 从文件名解析时间戳: progress.json.backup.1234567890
            if let Some(ts_str) = name.strip_prefix("progress.json.backup.") {
                if let Ok(file_timestamp) = ts_str.parse::<u64>() {
                    if file_timestamp < cutoff_timestamp {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    pub fn update_novel_progress(&mut self, novel_path: &Path, progress: ReadingProgress) {
        if let Some(novel) = self.novels.iter_mut().find(|n| n.path == novel_path) {
            novel.progress = progress;
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

    pub fn get_novel_progress(&self, novel_path: &Path) -> ReadingProgress {
        self.novels
            .iter()
            .find(|n| n.path == novel_path)
            .map(|n| n.progress.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
}
