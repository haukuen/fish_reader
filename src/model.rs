use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// 测试ReadingProgress的PartialEq实现
    #[test]
    fn test_reading_progress_equality() {
        let progress1 = ReadingProgress {
            line: 10,
            scroll_offset: 5,
        };
        let progress2 = ReadingProgress {
            line: 10,
            scroll_offset: 5,
        };
        let progress3 = ReadingProgress {
            line: 10,
            scroll_offset: 6,
        };

        assert_eq!(progress1, progress2);
        assert_ne!(progress1, progress3);
    }

    /// 测试标题提取功能
    #[test]
    fn test_novel_title_extraction() {
        // 测试普通文件名
        let novel1 = Novel::new(PathBuf::from("/path/to/novel.txt"));
        assert_eq!(novel1.title, "novel");

        // 测试中文文件名
        let novel2 = Novel::new(PathBuf::from("/path/to/红楼梦.txt"));
        assert_eq!(novel2.title, "红楼梦");

        // 测试复杂文件名
        let novel3 = Novel::new(PathBuf::from("/path/to/novel-part1_chapter1😄@@.txt"));
        assert_eq!(novel3.title, "novel-part1_chapter1😄@@");

        // 测试没有扩展名的文件
        let novel4 = Novel::new(PathBuf::from("/path/to/novel"));
        assert_eq!(novel4.title, "novel");
    }

    /// 测试加载的错误处理
    #[test]
    fn test_novel_content_loading_error() {
        let mut novel = Novel::new(PathBuf::from("/non/existent/file.txt"));
        let result = novel.load_content();

        // 应该返回错误
        assert!(result.is_err());
        // 内容应该保持为空
        assert!(novel.content.is_empty());
    }

    /// 测试加载空文件
    #[test]
    fn test_novel_content_loading_empty_file() -> std::io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("empty_novel.txt");
        fs::write(&file_path, "")?; // 创建空文件

        let mut novel = Novel::new(file_path);
        novel.load_content()?;

        assert_eq!(novel.content, "");
        Ok(())
    }

    /// 测试加载大文件
    #[test]
    fn test_novel_content_loading_large_file() -> std::io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("large_novel.txt");

        // 创建一个较大的文件
        let content = (0..1000000)
            .map(|i| format!("这是第{}行的内容，包含一些中文字符和数字123", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file_path, &content)?;

        let mut novel = Novel::new(file_path);
        novel.load_content()?;

        assert_eq!(novel.content, content);
        assert!(novel.content.lines().count() == 1000000);
        Ok(())
    }

    /// 测试Library的默认构造
    #[test]
    fn test_library_new() {
        let library = Library::new();
        assert!(library.novels.is_empty());
    }

    /// 测试Library从不存在的文件加载
    #[test]
    fn test_library_load_nonexistent_file() {
        // 测试从不存在的路径加载JSON文件的错误处理
        let non_existent_path = PathBuf::from("/tmp/non_existent_progress_12345.json");

        if non_existent_path.exists() {
            let _ = std::fs::remove_file(&non_existent_path);
        }

        let library = if non_existent_path.exists() {
            match std::fs::read_to_string(&non_existent_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(library) => library,
                    Err(_) => Library::new(),
                },
                Err(_) => Library::new(),
            }
        } else {
            Library::new()
        };

        // 应该返回一个新的空Library
        assert!(library.novels.is_empty());
    }

    /// 测试Library保存功能
    #[test]
    fn test_library_save() -> std::io::Result<()> {
        let dir = tempdir()?;
        let progress_file = dir.path().join("test_progress.json");

        let mut library = Library::new();
        let novel_path = PathBuf::from("/tmp/test_novel.txt");
        let progress = ReadingProgress {
            line: 42,
            scroll_offset: 10,
        };

        library.update_novel_progress(&novel_path, progress);

        // 手动保存到指定文件
        let content = serde_json::to_string_pretty(&library)?;
        fs::write(&progress_file, content)?;

        // 验证文件存在且内容正确
        assert!(progress_file.exists());
        let saved_content = fs::read_to_string(&progress_file)?;
        assert!(saved_content.contains("test_novel.txt"));
        assert!(saved_content.contains("42"));
        assert!(saved_content.contains("10"));

        Ok(())
    }

    /// 测试Library更新多个小说进度
    #[test]
    fn test_library_multiple_novels() {
        let mut library = Library::new();

        let novel1_path = PathBuf::from("/tmp/novel1.txt");
        let novel2_path = PathBuf::from("/tmp/novel2.txt");
        let novel3_path = PathBuf::from("/tmp/novel3.txt");

        let progress1 = ReadingProgress {
            line: 10,
            scroll_offset: 5,
        };
        let progress2 = ReadingProgress {
            line: 20,
            scroll_offset: 15,
        };
        let progress3 = ReadingProgress {
            line: 30,
            scroll_offset: 25,
        };

        // 添加多个小说
        library.update_novel_progress(&novel1_path, progress1);
        library.update_novel_progress(&novel2_path, progress2);
        library.update_novel_progress(&novel3_path, progress3);

        assert_eq!(library.novels.len(), 3);
        assert_eq!(library.get_novel_progress(&novel1_path), progress1);
        assert_eq!(library.get_novel_progress(&novel2_path), progress2);
        assert_eq!(library.get_novel_progress(&novel3_path), progress3);
    }

    /// 测试Library处理相同路径的小说
    #[test]
    fn test_library_same_path_update() {
        let mut library = Library::new();
        let novel_path = PathBuf::from("/tmp/same_novel.txt");

        let progress1 = ReadingProgress {
            line: 10,
            scroll_offset: 5,
        };
        let progress2 = ReadingProgress {
            line: 20,
            scroll_offset: 15,
        };

        // 第一次添加
        library.update_novel_progress(&novel_path, progress1);
        assert_eq!(library.novels.len(), 1);
        assert_eq!(library.get_novel_progress(&novel_path), progress1);

        // 更新相同路径的进度
        library.update_novel_progress(&novel_path, progress2);
        assert_eq!(library.novels.len(), 1); // 数量不应该增加
        assert_eq!(library.get_novel_progress(&novel_path), progress2); // 进度应该更新
    }

    /// 测试Library处理特殊字符的文件名
    #[test]
    fn test_library_special_filename() {
        let mut library = Library::new();

        // 测试包含特殊字符的文件名
        let special_paths = vec![
            PathBuf::from("/tmp/小说 with spaces.txt"),
            PathBuf::from("/tmp/novel-with-dashes.txt"),
            PathBuf::from("/tmp/novel_with_underscores.txt"),
            PathBuf::from("/tmp/novel.with.dots.txt"),
            PathBuf::from("/tmp/红楼梦.txt"),
        ];

        for (i, path) in special_paths.iter().enumerate() {
            let progress = ReadingProgress {
                line: i * 10,
                scroll_offset: i * 5,
            };
            library.update_novel_progress(path, progress);
        }

        assert_eq!(library.novels.len(), special_paths.len());

        // 验证标题提取正确
        let titles: Vec<&str> = library.novels.iter().map(|n| n.title.as_str()).collect();
        assert!(titles.contains(&"小说 with spaces"));
        assert!(titles.contains(&"novel-with-dashes"));
        assert!(titles.contains(&"novel_with_underscores"));
        assert!(titles.contains(&"novel.with.dots"));
        assert!(titles.contains(&"红楼梦"));
    }

    /// 测试Library的JSON序列化和反序列化边界情况
    #[test]
    fn test_library_json_edge_cases() -> std::io::Result<()> {
        let dir = tempdir()?;
        let progress_file = dir.path().join("edge_case_progress.json");

        let mut library = Library::new();

        // 添加边界值的进度
        let novel_path = PathBuf::from("/tmp/edge_case_novel.txt");
        let progress = ReadingProgress {
            line: usize::MAX,
            scroll_offset: 0,
        };
        library.update_novel_progress(&novel_path, progress);

        // 序列化
        let serialized = serde_json::to_string_pretty(&library)?;
        fs::write(&progress_file, &serialized)?;

        // 反序列化
        let content = fs::read_to_string(&progress_file)?;
        let deserialized: Library = serde_json::from_str(&content)?;

        assert_eq!(deserialized.novels.len(), 1);
        assert_eq!(
            deserialized.get_novel_progress(&novel_path).line,
            usize::MAX
        );
        assert_eq!(
            deserialized.get_novel_progress(&novel_path).scroll_offset,
            0
        );

        Ok(())
    }

    /// 测试Library处理损坏的JSON文件
    #[test]
    fn test_library_corrupted_json() -> std::io::Result<()> {
        let dir = tempdir()?;
        let progress_file = dir.path().join("corrupted_progress.json");

        // 写入损坏的JSON
        fs::write(&progress_file, "{ invalid json content }")?;

        // 尝试从损坏的文件加载应该返回新的Library
        // 注意：这里我们不能直接测试Library::load()，因为它使用固定路径
        // 但我们可以测试JSON反序列化的错误处理
        let content = fs::read_to_string(&progress_file)?;
        let result: Result<Library, _> = serde_json::from_str(&content);

        assert!(result.is_err());

        Ok(())
    }
}
