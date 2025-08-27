use anyhow::Result;
use ratatui::prelude::*;
use std::path::{Path, PathBuf};

use crate::model::library::{Library, NovelInfo};
use crate::model::novel::Novel;
use crate::state::{AppState, SettingsMode};

pub struct App {
    /// 当前应用状态（书架/阅读/搜索/章节目录模式）
    pub state: AppState,
    /// 持久化存储处理器
    pub library: Library,
    /// 发现的小说列表
    pub novels: Vec<Novel>,
    /// 书架选中的小说索引
    pub selected_novel_index: Option<usize>,
    /// 当前正在阅读的小说
    pub current_novel: Option<Novel>,
    /// 退出标志位
    pub should_quit: bool,
    /// 终端尺寸缓存
    pub terminal_size: Rect,
    /// 搜索输入框内容
    pub search_input: String,
    /// 搜索结果列表（行号，内容）
    pub search_results: Vec<(usize, String)>,
    /// 当前选中的搜索结果索引
    pub selected_search_index: Option<usize>,
    /// 当前选中的章节索引
    pub selected_chapter_index: Option<usize>,
    /// 上一个状态（用于从搜索/章节目录返回）
    pub previous_state: AppState,
    /// 孤立的小说记录（JSON中存在但文件已删除）
    pub orphaned_novels: Vec<NovelInfo>,
    /// 设置页面中选中的孤立小说索引
    pub selected_orphaned_index: Option<usize>,
    /// 设置界面的当前模式
    pub settings_mode: SettingsMode,
    /// 设置主菜单选中的选项索引
    pub selected_settings_option: Option<usize>,
    /// 删除小说模式下选中的小说索引
    pub selected_delete_novel_index: Option<usize>,
}

impl App {
    /// 初始化应用程序
    /// # 流程
    /// 1. 加载历史进度 2. 扫描小说目录
    pub fn new() -> Result<Self> {
        // 加载阅读进度
        let library = Library::load();

        // 获取小说文件
        let novels_dir = Self::get_novels_dir();
        let novels = Self::load_novels_from_dir(&novels_dir)?;

        let mut app = App {
            state: AppState::Bookshelf,
            library,
            novels,
            selected_novel_index: None,
            current_novel: None,
            should_quit: false,
            terminal_size: Rect::default(),
            search_input: String::new(),
            search_results: Vec::new(),
            selected_search_index: None,
            selected_chapter_index: None,
            previous_state: AppState::Bookshelf,
            orphaned_novels: Vec::new(),
            selected_orphaned_index: None,
            settings_mode: SettingsMode::MainMenu,
            selected_settings_option: None,
            selected_delete_novel_index: None,
        };

        // 检测孤立的小说记录
        app.detect_orphaned_novels();

        Ok(app)
    }

    pub fn get_novels_dir() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".fish_reader");
        path.push("novels");

        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
        }

        path
    }

    fn load_novels_from_dir(dir: &Path) -> Result<Vec<Novel>> {
        let mut novels = Vec::new();

        if !dir.exists() {
            return Ok(novels);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                let mut novel = Novel::new(path);
                let _ = novel.load_content(); // 忽略加载错误，继续处理其他小说
                novels.push(novel);
            }
        }

        Ok(novels)
    }

    /// 执行搜索操作
    /// # 功能
    /// 在当前小说内容中搜索包含关键词的行
    pub fn perform_search(&mut self) {
        if let Some(novel) = &self.current_novel {
            if !self.search_input.is_empty() {
                let lines: Vec<&str> = novel.content.lines().collect();
                self.search_results.clear();

                for (line_num, line) in lines.iter().enumerate() {
                    if line
                        .to_lowercase()
                        .contains(&self.search_input.to_lowercase())
                    {
                        self.search_results.push((line_num, line.to_string()));
                    }
                }

                // 更新选中索引，确保不越界
                if !self.search_results.is_empty() {
                    // 如果之前没有选中或选中索引越界，则选中第一个
                    if self.selected_search_index.is_none()
                        || self.selected_search_index.unwrap() >= self.search_results.len()
                    {
                        self.selected_search_index = Some(0);
                    }
                } else {
                    // 没有搜索结果时清空选中
                    self.selected_search_index = None;
                }
            } else {
                // 搜索输入为空时清空结果
                self.search_results.clear();
                self.selected_search_index = None;
            }
        }
    }

    /// 根据当前阅读位置找到最接近的章节索引
    /// # 返回
    /// 返回最接近当前阅读位置的章节索引，如果没有章节则返回None
    pub fn find_current_chapter_index(&self) -> Option<usize> {
        if let Some(novel) = &self.current_novel {
            if novel.chapters.is_empty() {
                return None;
            }

            let current_line = novel.progress.scroll_offset;
            let mut best_index = 0;

            // 找到当前阅读位置之前的最后一个章节
            for (index, chapter) in novel.chapters.iter().enumerate() {
                if chapter.start_line <= current_line {
                    best_index = index;
                } else {
                    break;
                }
            }

            Some(best_index)
        } else {
            None
        }
    }

    /// 检测孤立的小说记录（JSON中存在但文件已删除）
    /// # 功能
    /// 扫描library中的所有小说记录，找出文件已被删除的记录
    pub fn detect_orphaned_novels(&mut self) {
        self.orphaned_novels.clear();

        for novel_info in &self.library.novels {
            if !novel_info.path.exists() {
                self.orphaned_novels.push(novel_info.clone());
            }
        }

        // 重置选中索引
        self.selected_orphaned_index = None;
    }

    /// 删除选中的小说文件和进度记录
    /// # 参数
    /// - `index`: 要删除的小说在novels列表中的索引
    /// # 功能
    /// 1. 删除物理文件
    /// 2. 从novels列表中移除
    /// 3. 从library中移除进度记录
    /// 4. 保存library更改
    pub fn delete_novel(&mut self, index: usize) -> Result<()> {
        if index < self.novels.len() {
            let novel = &self.novels[index];

            if novel.path.exists() {
                std::fs::remove_file(&novel.path)?;
            }

            self.library.novels.retain(|n| n.path != novel.path);

            self.novels.remove(index);

            self.library.save()?;

            // 调整选中索引
            if !self.novels.is_empty() {
                let new_index = index.min(self.novels.len() - 1);
                self.selected_delete_novel_index = Some(new_index);
            } else {
                self.selected_delete_novel_index = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::novel::{Chapter, ReadingProgress};
    use std::path::PathBuf;

    fn create_test_app() -> App {
        App {
            state: AppState::Bookshelf,
            library: Library::default(),
            novels: Vec::new(),
            selected_novel_index: None,
            current_novel: None,
            should_quit: false,
            terminal_size: Rect::default(),
            search_input: String::new(),
            search_results: Vec::new(),
            selected_search_index: None,
            selected_chapter_index: None,
            previous_state: AppState::Bookshelf,
            orphaned_novels: Vec::new(),
            selected_orphaned_index: None,
            settings_mode: SettingsMode::MainMenu,
            selected_settings_option: None,
            selected_delete_novel_index: None,
        }
    }

    #[test]
    fn test_perform_search() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "Hello world\nThis is a test\nAnother TEST line".to_string();
        app.current_novel = Some(novel);

        app.search_input = "test".to_string();
        app.perform_search();
        assert_eq!(app.search_results.len(), 2);
        assert_eq!(app.search_results[0], (1, "This is a test".to_string()));
        assert_eq!(app.search_results[1], (2, "Another TEST line".to_string()));

        app.search_input = "".to_string();
        app.perform_search();
        assert!(app.search_results.is_empty());
    }

    #[test]
    fn test_find_current_chapter_index() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.chapters = vec![
            Chapter {
                title: "Intro".to_string(),
                start_line: 0,
            },
            Chapter {
                title: "Middle".to_string(),
                start_line: 10,
            },
            Chapter {
                title: "End".to_string(),
                start_line: 20,
            },
        ];
        app.current_novel = Some(novel);

        app.current_novel.as_mut().unwrap().progress = ReadingProgress { scroll_offset: 5 };
        assert_eq!(app.find_current_chapter_index(), Some(0));

        app.current_novel.as_mut().unwrap().progress = ReadingProgress { scroll_offset: 15 };
        assert_eq!(app.find_current_chapter_index(), Some(1));

        app.current_novel.as_mut().unwrap().progress = ReadingProgress { scroll_offset: 25 };
        assert_eq!(app.find_current_chapter_index(), Some(2));
    }
}
