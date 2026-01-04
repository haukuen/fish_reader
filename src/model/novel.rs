use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Novel {
    /// 小说标题（从文件名自动提取）
    pub title: String,
    /// 小说文件的绝对路径
    pub path: PathBuf,
    /// 小说文本内容
    pub content: String,
    /// 当前阅读进度
    pub progress: ReadingProgress,
    /// 章节目录
    pub chapters: Vec<Chapter>,
}

impl Novel {
    /// 创建新小说实例
    /// # 参数
    /// - `path`: 小说文件路径
    ///
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
            chapters: Vec::new(),
        }
    }

    pub fn load_content(&mut self) -> std::io::Result<()> {
        self.content = std::fs::read_to_string(&self.path)?;
        self.parse_chapters();
        Ok(())
    }

    /// 解析章节目录
    /// # 功能
    /// 从小说内容中自动识别章节标题，支持多种常见格式
    pub fn parse_chapters(&mut self) {
        self.chapters.clear();

        let lines: Vec<&str> = self.content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // 跳过空行
            if trimmed.is_empty() {
                continue;
            }

            // 检查是否为章节标题
            if self.is_chapter_title(trimmed) {
                self.chapters.push(Chapter {
                    title: trimmed.to_string(),
                    start_line: line_num,
                });
            }
        }
    }

    /// 判断一行文本是否为章节标题
    /// # 参数
    /// - `line`: 待检查的文本行
    /// # 返回
    /// 如果是章节标题返回true，否则返回false
    fn is_chapter_title(&self, line: &str) -> bool {
        let line = line.trim();

        // 检查常见的章节标题模式
        let chapter_keywords = ['章', '回', '节', '卷', '部', '篇'];
        if line.starts_with("第")
            && let Some(keyword_pos) = line.find(chapter_keywords)
        {
            let start_index = "第".len();
            // Ensure there is something between "第" and the keyword
            if keyword_pos > start_index {
                let number_part = &line[start_index..keyword_pos];
                // The part between "第" and the keyword should not contain whitespace
                if !number_part.chars().any(|c| c.is_whitespace()) {
                    return true;
                }
            }
        }

        // 检查英文章节
        if line.to_lowercase().starts_with("chapter") {
            return true;
        }

        // 检查特殊章节
        let special_chapters = [
            "序章", "序言", "楔子", "尾声", "后记", "番外", "终章", "结语", "引子", "开篇",
        ];
        for special in &special_chapters {
            if line.starts_with(special) {
                return true;
            }
        }

        // 检查数字+点号+章节名格式 (如 "001.网咖系统与看板娘")
        if let Some(dot_pos) = line.find('.')
            && dot_pos > 0
            && dot_pos < line.len() - 1
        {
            let number_part = &line[0..dot_pos];
            let title_part = &line[dot_pos + 1..];
            // 数字部分全是数字，标题部分不为空且包含字母或中文字符
            if number_part.chars().all(|c| c.is_ascii_digit())
                && !title_part.trim().is_empty()
                && title_part.trim().chars().any(|c| c.is_alphabetic())
                && !number_part.is_empty()
                && number_part.len() <= 6
            {
                return true;
            }
        }

        // 检查纯数字章节 (如 "1." "2、")
        if line.len() <= 10 {
            let chars: Vec<char> = line.chars().collect();
            if chars.len() >= 2 {
                let first_part = &chars[0..chars.len() - 1];
                let last_char = chars[chars.len() - 1];
                if first_part.iter().all(|c| c.is_ascii_digit())
                    && (last_char == '.' || last_char == '、')
                {
                    return true;
                }
            }
        }

        // 检查中文数字章节
        let chinese_numbers = [
            '一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '百', '千', '万',
        ];
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > 1 && chars.len() <= 10 {
            let last_char = chars[chars.len() - 1];
            if last_char == '、' || last_char == '.' {
                let first_part = &chars[0..chars.len() - 1];
                if first_part.iter().all(|c| chinese_numbers.contains(c)) {
                    return true;
                }
            }
        }

        false
    }
}

/// 章节信息结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chapter {
    /// 章节标题
    pub title: String,
    /// 章节在文本中的起始行号
    pub start_line: usize,
}

/// 书签信息结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    /// 书签名称
    pub name: String,
    /// 书签位置（行号）
    pub position: usize,
    /// 创建时间戳
    pub timestamp: u64,
}

impl Bookmark {
    /// 创建新书签
    /// # 参数
    /// - `name`: 书签名称
    /// - `position`: 书签位置（行号）
    pub fn new(name: String, position: usize) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Bookmark {
            name,
            position,
            timestamp,
        }
    }
}

/// 阅读进度跟踪结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReadingProgress {
    /// 滚动偏移量（用于界面渲染）
    pub scroll_offset: usize,
    /// 书签列表
    pub bookmarks: Vec<Bookmark>,
}

impl ReadingProgress {
    /// 添加书签
    /// # 参数
    /// - `name`: 书签名称
    /// - `position`: 书签位置（行号）
    pub fn add_bookmark(&mut self, name: String, position: usize) {
        let bookmark = Bookmark::new(name, position);
        self.bookmarks.push(bookmark);
        // 按位置排序书签
        self.bookmarks.sort_by(|a, b| a.position.cmp(&b.position));
    }

    /// 删除书签
    /// # 参数
    /// - `index`: 书签在列表中的索引
    pub fn remove_bookmark(&mut self, index: usize) -> Option<Bookmark> {
        if index < self.bookmarks.len() {
            Some(self.bookmarks.remove(index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_novel_new() {
        let path = PathBuf::from("/path/to/my_novel.txt");
        let novel = Novel::new(path);
        assert_eq!(novel.title, "my_novel");
    }

    #[test]
    fn test_is_chapter_title() {
        let novel = Novel::new(PathBuf::from("test.txt"));
        assert!(novel.is_chapter_title("第一章 标题"));
        assert!(novel.is_chapter_title("第100回"));
        assert!(novel.is_chapter_title("Chapter 1: The Beginning"));
        assert!(novel.is_chapter_title("序章"));
        assert!(novel.is_chapter_title("123."));
        assert!(novel.is_chapter_title("一二三、"));
        assert!(novel.is_chapter_title("001.网咖系统与看板娘"));
        assert!(novel.is_chapter_title("1.开始"));
        assert!(novel.is_chapter_title("999.结束"));
        assert!(!novel.is_chapter_title("This is a normal line."));
        assert!(!novel.is_chapter_title("第一 章"));
        assert!(!novel.is_chapter_title(".无数字开头"));
    }

    #[test]
    fn test_parse_chapters() {
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "序章
Some content

第一章 The Real Start
More content
Chapter 2
Final content"
            .to_string();
        novel.parse_chapters();

        assert_eq!(novel.chapters.len(), 3);
        assert_eq!(
            novel.chapters[0],
            Chapter {
                title: "序章".to_string(),
                start_line: 0
            }
        );
        assert_eq!(
            novel.chapters[1],
            Chapter {
                title: "第一章 The Real Start".to_string(),
                start_line: 3
            }
        );
        assert_eq!(
            novel.chapters[2],
            Chapter {
                title: "Chapter 2".to_string(),
                start_line: 5
            }
        );
    }

    #[test]
    fn test_reading_progress_add_bookmark() {
        let mut progress = ReadingProgress::default();
        progress.add_bookmark("Test Bookmark".to_string(), 100);

        assert_eq!(progress.bookmarks.len(), 1);
        assert_eq!(progress.bookmarks[0].name, "Test Bookmark");
        assert_eq!(progress.bookmarks[0].position, 100);
    }

    #[test]
    fn test_reading_progress_add_bookmarks_sorted() {
        let mut progress = ReadingProgress::default();
        progress.add_bookmark("Second".to_string(), 200);
        progress.add_bookmark("First".to_string(), 100);
        progress.add_bookmark("Third".to_string(), 300);

        // 应按位置排序
        assert_eq!(progress.bookmarks.len(), 3);
        assert_eq!(progress.bookmarks[0].position, 100);
        assert_eq!(progress.bookmarks[0].name, "First");
        assert_eq!(progress.bookmarks[1].position, 200);
        assert_eq!(progress.bookmarks[1].name, "Second");
        assert_eq!(progress.bookmarks[2].position, 300);
        assert_eq!(progress.bookmarks[2].name, "Third");
    }

    #[test]
    fn test_reading_progress_remove_bookmark() {
        let mut progress = ReadingProgress::default();
        progress.add_bookmark("Test".to_string(), 100);

        let removed = progress.remove_bookmark(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Test");
        assert!(progress.bookmarks.is_empty());
    }

    #[test]
    fn test_reading_progress_remove_bookmark_invalid_index() {
        let mut progress = ReadingProgress::default();
        let removed = progress.remove_bookmark(99);
        assert!(removed.is_none());
    }

    #[test]
    fn test_bookmark_new() {
        let bookmark = Bookmark::new("Test".to_string(), 42);
        assert_eq!(bookmark.name, "Test");
        assert_eq!(bookmark.position, 42);
        assert!(bookmark.timestamp > 0);
    }

    #[test]
    fn test_is_chapter_title_edge_cases() {
        let novel = Novel::new(PathBuf::from("test.txt"));

        // 边界情况
        assert!(!novel.is_chapter_title(""));
        assert!(!novel.is_chapter_title("   "));
        assert!(!novel.is_chapter_title("第"));
        assert!(novel.is_chapter_title("后记"));
        assert!(novel.is_chapter_title("番外"));
        assert!(novel.is_chapter_title("楔子"));
        assert!(novel.is_chapter_title("尾声"));
    }
}
