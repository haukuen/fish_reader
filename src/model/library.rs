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
    pub path: PathBuf,
    pub progress: ReadingProgress,
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

    fn create_backup_if_needed(progress_path: &Path) -> std::io::Result<()> {
        if !progress_path.exists() {
            return Ok(());
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let period_timestamp = timestamp / CONFIG.backup_timestamp_interval * CONFIG.backup_timestamp_interval;

        // 直接在文件名后追加备份后缀，避免 with_extension 替换原有扩展名
        let file_name = progress_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(CONFIG.progress_filename);
        let backup_name = format!("{}.{}.{}", file_name, CONFIG.backup_suffix, period_timestamp);
        let backup_path = progress_path.with_file_name(backup_name);

        if backup_path.exists() {
            return Ok(());
        }

        std::fs::copy(progress_path, &backup_path)?;

        let cutoff_timestamp = timestamp.saturating_sub(CONFIG.backup_retention_days * 24 * 60 * 60);
        if let Some(backup_dir) = progress_path.parent() {
            Self::cleanup_old_backups(backup_dir, cutoff_timestamp);
        }

        Ok(())
    }

    fn cleanup_old_backups(backup_dir: &Path, cutoff_timestamp: u64) {
        let Ok(entries) = std::fs::read_dir(backup_dir) else {
            return;
        };

        // 备份文件名格式: {progress_filename}.{backup_suffix}.{timestamp}
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

    /// 更新或添加小说的阅读进度
    ///
    /// 如果小说已存在则更新进度，否则创建新条目。
    ///
    /// # Arguments
    ///
    /// * `novel_path` - 小说文件路径
    /// * `progress` - 阅读进度
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
