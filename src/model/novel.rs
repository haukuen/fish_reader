use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        if line.starts_with("第") {
            if let Some(keyword_pos) = line.find(chapter_keywords) {
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

/// 阅读进度跟踪结构
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct ReadingProgress {
    /// 滚动偏移量（用于界面渲染）
    pub scroll_offset: usize,
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
        assert!(!novel.is_chapter_title("This is a normal line."));
        assert!(!novel.is_chapter_title("第一 章")); // space
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
}
