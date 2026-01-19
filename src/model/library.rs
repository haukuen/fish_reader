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
                    Err(_) => return Self::new(),
                },
                Err(_) => return Self::new(),
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
        std::fs::write(progress_path, content)?;
        Ok(())
    }

    pub fn get_progress_path() -> PathBuf {
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
