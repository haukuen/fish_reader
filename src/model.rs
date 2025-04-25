use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    /// 小说标题（从文件名自动提取）
    pub title: String,
    /// 小说文件的绝对路径
    pub path: PathBuf,
    /// 小说文本内容（懒加载）
    pub content: String,
    /// 当前阅读进度
    pub progress: ReadingProgress,
}

impl Novel {
    /// 创建新小说实例
    /// # 参数
    /// - `path`: 小说文件路径  
    /// # 注意
    /// 不会立即加载文件内容，需要显式调用load_content
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("未知标题")
            .to_string();

        Novel {
            title,
            path: path.clone(),
            content: String::new(),
            progress: ReadingProgress::default(),
        }
    }

    pub fn load_content(&mut self) -> std::io::Result<()> {
        self.content = std::fs::read_to_string(&self.path)?;
        Ok(())
    }
}

/// 阅读进度跟踪结构
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct ReadingProgress {
    /// 当前阅读行号（基于0的索引）
    pub line: usize,
    /// 滚动偏移量（用于界面渲染）
    pub scroll_offset: usize,
}

/// 管理用户的小说库和阅读进度
#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    /// 所有已跟踪的小说信息
    pub novels: Vec<NovelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
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
            .map(|n| n.progress)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_novel_creation() {
        let path = PathBuf::from("/tmp/test_novel.txt");
        let novel = Novel::new(path.clone());

        assert_eq!(novel.title, "test_novel");
        assert_eq!(novel.path, path);
        assert!(novel.content.is_empty());
        assert_eq!(novel.progress.line, 0);
        assert_eq!(novel.progress.scroll_offset, 0);
    }

    #[test]
    fn test_novel_content_loading() -> std::io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_novel.txt");
        let content = "Line 1\nLine 2\nLine 3";
        fs::write(&file_path, content)?;

        let mut novel = Novel::new(file_path);
        novel.load_content()?;

        assert_eq!(novel.content, content);
        Ok(())
    }

    #[test]
    fn test_library_progress_management() {
        let mut library = Library::new();
        let novel_path = PathBuf::from("/tmp/test_novel.txt");
        let progress = ReadingProgress {
            line: 42,
            scroll_offset: 10,
        };

        // Test updating progress for a new novel
        library.update_novel_progress(&novel_path, progress);
        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.get_novel_progress(&novel_path), progress);

        // Test updating progress for existing novel
        let new_progress = ReadingProgress {
            line: 50,
            scroll_offset: 15,
        };
        library.update_novel_progress(&novel_path, new_progress);
        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.get_novel_progress(&novel_path), new_progress);

        // Test getting progress for non-existent novel
        let non_existent = PathBuf::from("/tmp/non_existent.txt");
        let default_progress = library.get_novel_progress(&non_existent);
        assert_eq!(default_progress.line, 0);
        assert_eq!(default_progress.scroll_offset, 0);
    }

    #[test]
    fn test_library_serialization() -> std::io::Result<()> {
        let dir = tempdir()?;
        let progress_file = dir.path().join("progress.json");

        let mut library = Library::new();
        let novel_path = PathBuf::from("/tmp/test_novel.txt");
        let progress = ReadingProgress {
            line: 42,
            scroll_offset: 10,
        };

        library.update_novel_progress(&novel_path, progress);

        // 写入文件
        let serialized = serde_json::to_string_pretty(&library)?;
        fs::write(&progress_file, &serialized)?;

        // 从文件读取
        let content = fs::read_to_string(&progress_file)?;
        let deserialized: Library = serde_json::from_str(&content)?;

        assert_eq!(deserialized.novels.len(), library.novels.len());
        assert_eq!(
            deserialized.get_novel_progress(&novel_path),
            library.get_novel_progress(&novel_path)
        );

        Ok(())
    }

    #[test]
    fn test_reading_progress_default() {
        let progress = ReadingProgress::default();
        assert_eq!(progress.line, 0);
        assert_eq!(progress.scroll_offset, 0);
    }
}
