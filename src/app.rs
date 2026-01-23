use anyhow::Result;
use ratatui::prelude::*;
use std::path::{Path, PathBuf};

use crate::config::CONFIG;
use crate::model::library::{Library, NovelInfo};
use crate::model::novel::Novel;
use crate::state::{AppState, SettingsMode};

/// 搜索相关状态
#[derive(Default)]
pub struct SearchState {
    /// 搜索输入框内容
    pub input: String,
    /// 搜索结果列表（行号，内容）
    pub results: Vec<(usize, String)>,
    /// 当前选中的搜索结果索引
    pub selected_index: Option<usize>,
}

impl SearchState {
    /// 清空搜索状态
    ///
    /// 重置输入框、搜索结果和选中索引。
    pub fn clear(&mut self) {
        self.input.clear();
        self.results.clear();
        self.selected_index = None;
    }
}

/// 书签相关状态
#[derive(Default)]
pub struct BookmarkState {
    /// 当前选中的书签索引
    pub selected_index: Option<usize>,
    /// 添加书签时的输入内容
    pub input: String,
}

impl BookmarkState {
    /// 清空输入框内容
    pub fn clear_input(&mut self) {
        self.input.clear();
    }
}

/// 设置相关状态
#[derive(Default)]
pub struct SettingsState {
    /// 设置界面的当前模式
    pub mode: SettingsMode,
    /// 设置主菜单选中的选项索引
    pub selected_option: Option<usize>,
    /// 删除小说模式下选中的小说索引
    pub selected_delete_novel_index: Option<usize>,
    /// 孤立的小说记录（JSON中存在但文件已删除）
    pub orphaned_novels: Vec<NovelInfo>,
    /// 设置页面中选中的孤立小说索引
    pub selected_orphaned_index: Option<usize>,
}

impl SettingsState {
    /// 重置设置状态到主菜单
    pub fn reset(&mut self) {
        self.mode = SettingsMode::MainMenu;
        self.selected_option = None;
    }
}

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
    /// 当前选中的章节索引
    pub selected_chapter_index: Option<usize>,
    /// 上一个状态（用于从搜索/章节目录返回）
    pub previous_state: AppState,

    /// 搜索状态
    pub search: SearchState,
    /// 书签状态
    pub bookmark: BookmarkState,
    /// 设置状态
    pub settings: SettingsState,
    /// 错误消息（用于在状态栏显示错误提示）
    pub error_message: Option<String>,
}

impl App {
    /// 初始化应用程序
    /// # 流程
    /// 1. 加载历史进度 2. 扫描小说目录（懒加载，不加载内容）
    pub fn new() -> Result<Self> {
        // 加载阅读进度
        let library = Library::load();

        // 获取小说文件（懒加载，只扫描文件不加载内容）
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
            selected_chapter_index: None,
            previous_state: AppState::Bookshelf,
            search: SearchState::default(),
            bookmark: BookmarkState::default(),
            settings: SettingsState::default(),
            error_message: None,
        };

        // 检测孤立的小说记录
        app.detect_orphaned_novels();

        Ok(app)
    }

    /// 获取小说存储目录路径
    ///
    /// # Returns
    ///
    /// 小说目录的完整路径。在测试环境下返回临时目录，否则返回用户主目录下的 `.fish_reader/novels`。
    pub fn get_novels_dir() -> PathBuf {
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
            let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push(CONFIG.dir_name);
            path.push("novels");

            if !path.exists()
                && let Err(e) = std::fs::create_dir_all(&path)
            {
                eprintln!("Failed to create directory: {}", e);
            }

            path
        }
    }

    /// 从指定目录扫描并加载小说列表
    ///
    /// 仅扫描支持的文件扩展名，采用懒加载策略（不加载文件内容）。
    ///
    /// # Arguments
    ///
    /// * `dir` - 要扫描的目录路径
    ///
    /// # Returns
    ///
    /// 返回找到的小说列表，如果目录不存在则返回空列表。
    ///
    /// # Errors
    ///
    /// 如果目录读取失败则返回错误。
    fn load_novels_from_dir(dir: &Path) -> Result<Vec<Novel>> {
        let mut novels = Vec::new();

        if !dir.exists() {
            return Ok(novels);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
                    && CONFIG.supported_extensions.contains(&ext) {
                        // 懒加载：只创建 Novel 对象，不加载文件内容
                        let novel = Novel::new(path);
                        novels.push(novel);
                    }
        }

        Ok(novels)
    }

    /// 在当前小说内容中搜索关键词
    ///
    /// 执行不区分大小写的搜索，更新搜索结果列表。
    ///
    /// # Note
    ///
    /// 搜索输入为空时会清空结果列表。
    pub fn perform_search(&mut self) {
        if let Some(novel) = &self.current_novel {
            if !self.search.input.is_empty() {
                let lines: Vec<&str> = novel.content.lines().collect();
                self.search.results.clear();

                for (line_num, line) in lines.iter().enumerate() {
                    if line
                        .to_lowercase()
                        .contains(&self.search.input.to_lowercase())
                    {
                        self.search.results.push((line_num, line.to_string()));
                    }
                }

                // 更新选中索引，确保不越界
                if !self.search.results.is_empty() {
                    // 如果之前没有选中或选中索引越界，则选中第一个
                    let should_reset = match self.search.selected_index {
                        None => true,
                        Some(idx) => idx >= self.search.results.len(),
                    };
                    if should_reset {
                        self.search.selected_index = Some(0);
                    }
                } else {
                    // 没有搜索结果时清空选中
                    self.search.selected_index = None;
                }
            } else {
                // 搜索输入为空时清空结果
                self.search.results.clear();
                self.search.selected_index = None;
            }
        }
    }

    /// 根据当前阅读位置查找对应的章节索引
    ///
    /// # Returns
    ///
    /// 返回最接近当前阅读位置的章节索引。如果没有章节或当前未打开小说，返回 `None`。
    pub fn find_current_chapter_index(&self) -> Option<usize> {
        self.current_novel.as_ref().and_then(|novel| {
            if novel.chapters.is_empty() {
                return None;
            }
            Some(Self::find_chapter_index(&novel.chapters, novel.progress.scroll_offset))
        })
    }

    /// 检测孤立的小说记录
    ///
    /// 扫描 library 中所有小说记录，找出 JSON 中存在但文件已被删除的记录。
    pub fn detect_orphaned_novels(&mut self) {
        self.settings.orphaned_novels.clear();

        for novel_info in &self.library.novels {
            if !novel_info.path.exists() {
                self.settings.orphaned_novels.push(novel_info.clone());
            }
        }

        // 重置选中索引
        self.settings.selected_orphaned_index = None;
    }

    /// 删除指定索引的小说
    ///
    /// 执行以下操作：
    /// 1. 删除物理文件
    /// 2. 从 novels 列表中移除
    /// 3. 从 library 中移除进度记录
    /// 4. 保存 library 更改
    ///
    /// # Arguments
    ///
    /// * `index` - 要删除的小说在 novels 列表中的索引
    ///
    /// # Errors
    ///
    /// 如果文件删除或保存失败则返回错误。
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
                self.settings.selected_delete_novel_index = Some(new_index);
            } else {
                self.settings.selected_delete_novel_index = None;
            }
        }
        Ok(())
    }

    /// 在当前小说的阅读位置添加书签
    ///
    /// # Arguments
    ///
    /// * `name` - 书签名称
    pub fn add_bookmark(&mut self, name: String) {
        if let Some(novel) = &mut self.current_novel {
            let position = novel.progress.scroll_offset;
            novel.progress.add_bookmark(name, position);
            self.save_current_progress();
        }
    }

    /// 删除当前小说的指定书签
    ///
    /// # Arguments
    ///
    /// * `index` - 要删除的书签索引
    ///
    /// # Returns
    ///
    /// 如果删除成功返回 `Some(())`，如果索引无效或当前无小说则返回 `None`。
    pub fn remove_bookmark(&mut self, index: usize) -> Option<()> {
        if let Some(novel) = &mut self.current_novel
            && novel.progress.remove_bookmark(index).is_some()
        {
            self.save_current_progress();
            Some(())
        } else {
            None
        }
    }

    /// 跳转到指定书签位置
    ///
    /// # Arguments
    ///
    /// * `index` - 要跳转的书签索引
    ///
    /// # Returns
    ///
    /// 如果跳转成功返回 `Some(())`，如果索引无效或当前无小说则返回 `None`。
    pub fn jump_to_bookmark(&mut self, index: usize) -> Option<()> {
        if let Some(novel) = &mut self.current_novel
            && let Some(bookmark) = novel.progress.bookmarks.get(index)
        {
            novel.progress.scroll_offset = bookmark.position;
            self.save_current_progress();
            Some(())
        } else {
            None
        }
    }

    /// 获取当前小说的书签列表
    ///
    /// # Returns
    ///
    /// 如果当前有打开的小说则返回其书签列表的引用，否则返回 `None`。
    pub fn get_current_bookmarks(&self) -> Option<&Vec<crate::model::novel::Bookmark>> {
        self.current_novel
            .as_ref()
            .map(|novel| &novel.progress.bookmarks)
    }

    /// 清空书签输入框内容
    pub fn clear_bookmark_inputs(&mut self) {
        self.bookmark.clear_input();
    }

    /// 设置错误消息
    ///
    /// 错误消息将在下一帧渲染时显示给用户。
    ///
    /// # Arguments
    ///
    /// * `msg` - 错误消息内容
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error_message = Some(msg.into());
    }

    /// 保存当前小说的阅读进度
    ///
    /// 更新并保存当前小说的进度。如果保存失败，会设置错误消息。
    pub fn save_current_progress(&mut self) {
        if let Some(novel) = &self.current_novel {
            self.library
                .update_novel_progress(&novel.path, novel.progress.clone());
            if let Err(e) = self.library.save() {
                self.set_error(format!("Failed to save progress: {}", e));
            }
        }
    }

    /// 查找指定行所在的章节索引
    ///
    /// # Arguments
    ///
    /// * `chapters` - 章节列表
    /// * `current_line` - 当前行号
    ///
    /// # Returns
    ///
    /// 最接近当前行的章节索引（即 `start_line` 小于等于 `current_line` 的最大索引）。
    pub fn find_chapter_index(
        chapters: &[crate::model::novel::Chapter],
        current_line: usize,
    ) -> usize {
        let mut current_idx = 0;
        for (index, chapter) in chapters.iter().enumerate() {
            if chapter.start_line <= current_line {
                current_idx = index;
            } else {
                break;
            }
        }
        current_idx
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
            selected_chapter_index: None,
            previous_state: AppState::Bookshelf,
            search: SearchState::default(),
            bookmark: BookmarkState::default(),
            settings: SettingsState::default(),
            error_message: None,
        }
    }

    #[test]
    fn test_perform_search() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content =
            std::sync::Arc::new("Hello world\nThis is a test\nAnother TEST line".to_string());
        app.current_novel = Some(novel);

        app.search.input = "test".to_string();
        app.perform_search();
        assert_eq!(app.search.results.len(), 2);
        assert_eq!(app.search.results[0], (1, "This is a test".to_string()));
        assert_eq!(app.search.results[1], (2, "Another TEST line".to_string()));

        app.search.input = "".to_string();
        app.perform_search();
        assert!(app.search.results.is_empty());
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

        app.current_novel.as_mut().unwrap().progress = ReadingProgress {
            scroll_offset: 5,
            bookmarks: Vec::new(),
        };
        assert_eq!(app.find_current_chapter_index(), Some(0));

        app.current_novel.as_mut().unwrap().progress = ReadingProgress {
            scroll_offset: 15,
            bookmarks: Vec::new(),
        };
        assert_eq!(app.find_current_chapter_index(), Some(1));

        app.current_novel.as_mut().unwrap().progress = ReadingProgress {
            scroll_offset: 25,
            bookmarks: Vec::new(),
        };
        assert_eq!(app.find_current_chapter_index(), Some(2));
    }

    #[test]
    fn test_add_bookmark() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.progress.scroll_offset = 50;
        app.current_novel = Some(novel);

        app.add_bookmark("My Bookmark".to_string());

        let bookmarks = app.get_current_bookmarks().unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].name, "My Bookmark");
        assert_eq!(bookmarks[0].position, 50);
    }

    #[test]
    fn test_remove_bookmark() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.progress.add_bookmark("Test".to_string(), 10);
        app.current_novel = Some(novel);

        let result = app.remove_bookmark(0);
        assert!(result.is_some());
        assert!(app.get_current_bookmarks().unwrap().is_empty());
    }

    #[test]
    fn test_remove_bookmark_no_novel() {
        let mut app = create_test_app();
        let result = app.remove_bookmark(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_jump_to_bookmark() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.progress.add_bookmark("Target".to_string(), 100);
        app.current_novel = Some(novel);

        let result = app.jump_to_bookmark(0);
        assert!(result.is_some());
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            100
        );
    }

    #[test]
    fn test_jump_to_bookmark_invalid_index() {
        let mut app = create_test_app();
        let novel = Novel::new(PathBuf::from("test.txt"));
        app.current_novel = Some(novel);

        let result = app.jump_to_bookmark(99);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_current_bookmarks_no_novel() {
        let app = create_test_app();
        assert!(app.get_current_bookmarks().is_none());
    }

    #[test]
    fn test_clear_bookmark_inputs() {
        let mut app = create_test_app();
        app.bookmark.input = "some input".to_string();
        app.clear_bookmark_inputs();
        assert!(app.bookmark.input.is_empty());
    }

    #[test]
    fn test_search_state_clear() {
        let mut search = SearchState::default();
        search.input = "query".to_string();
        search.results = vec![(1, "result".to_string())];
        search.selected_index = Some(0);

        search.clear();

        assert!(search.input.is_empty());
        assert!(search.results.is_empty());
        assert!(search.selected_index.is_none());
    }

    #[test]
    fn test_settings_state_reset() {
        let mut settings = SettingsState::default();
        settings.mode = SettingsMode::DeleteNovel;
        settings.selected_option = Some(5);

        settings.reset();

        assert_eq!(settings.mode, SettingsMode::MainMenu);
        assert!(settings.selected_option.is_none());
    }

    #[test]
    fn test_find_current_chapter_index_no_novel() {
        let app = create_test_app();
        assert!(app.find_current_chapter_index().is_none());
    }

    #[test]
    fn test_find_current_chapter_index_no_chapters() {
        let mut app = create_test_app();
        let novel = Novel::new(PathBuf::from("test.txt"));
        app.current_novel = Some(novel);
        assert!(app.find_current_chapter_index().is_none());
    }
}
