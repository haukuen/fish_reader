use anyhow::Result;
use ratatui::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use crate::config::CONFIG;
use crate::model::library::{Library, NovelInfo};
use crate::model::novel::Novel;
use crate::state::{AppState, SettingsMode};
use crate::sync::config::WebDavConfig;
use crate::sync::sync_engine::SyncMessage;
use crate::ui::sync_status::SyncStatus;

mod bookmark;
mod library_ops;
mod search;
mod sync_ops;

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
    /// WebDAV配置编辑状态
    pub webdav_config_state: WebDavConfigState,
}

/// WebDAV配置编辑状态
#[derive(Default)]
pub struct WebDavConfigState {
    /// 当前选中的字段索引 (0=enabled, 1=url, 2=username, 3=password, 4=remote_path)
    pub selected_field: usize,
    /// 临时配置（编辑中）
    pub temp_config: WebDavConfig,
    /// 是否处于编辑模式（输入文本时）
    pub edit_mode: bool,
    /// 是否显示密码（明文/密文）
    pub show_password: bool,
    /// 连接测试结果 (None=未测试, Ok=成功, Err=失败原因)
    pub connection_status: Option<Result<(), String>>,
}

impl SettingsState {
    /// 重置设置状态到主菜单
    pub fn reset(&mut self) {
        self.mode = SettingsMode::MainMenu;
        self.selected_option = None;
        self.webdav_config_state = WebDavConfigState::default();
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

    /// WebDAV 配置
    pub webdav_config: WebDavConfig,
    /// 同步消息接收端（后台线程通信）
    pub sync_rx: Option<Receiver<SyncMessage>>,
    /// 同步状态显示
    pub sync_status: SyncStatus,
}

impl App {
    #[cfg(test)]
    fn get_test_data_dir() -> PathBuf {
        let mut path = std::env::temp_dir();
        let thread_id = format!("{:?}", std::thread::current().id())
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        path.push(format!(
            "{}_test_{}_{}",
            CONFIG.dir_name,
            std::process::id(),
            thread_id
        ));
        path
    }

    /// 初始化应用程序
    /// # 流程
    /// 1. 加载历史进度 2. 扫描小说目录（懒加载，不加载内容）
    pub fn new() -> Result<Self> {
        let library = Library::load();

        let novels_dir = Self::get_novels_dir();
        let novels = Self::load_novels_from_dir(&novels_dir)?;

        let webdav_config = WebDavConfig::load();

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
            webdav_config,
            sync_rx: None,
            sync_status: SyncStatus::Idle,
        };

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
            let mut path = Self::get_test_data_dir();
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
                && CONFIG.supported_extensions.contains(&ext)
            {
                let novel = Novel::new(path);
                novels.push(novel);
            }
        }

        novels.sort_by(|a, b| {
            a.title
                .to_lowercase()
                .cmp(&b.title.to_lowercase())
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.path.cmp(&b.path))
        });

        Ok(novels)
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

    /// Save WebDAV configuration
    pub fn save_webdav_config(&mut self) {
        self.webdav_config = self.settings.webdav_config_state.temp_config.clone();

        if let Err(e) = self.webdav_config.save() {
            self.set_error(format!("Failed to save WebDAV config: {}", e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::novel::{Chapter, ReadingProgress};
    use std::path::PathBuf;
    use std::sync::mpsc;
    use tempfile::tempdir;

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
            webdav_config: WebDavConfig::default(),
            sync_rx: None,
            sync_status: SyncStatus::Idle,
        }
    }

    #[test]
    fn test_perform_search() {
        let mut app = create_test_app();
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        let content = "Hello world\nThis is a test\nAnother TEST line".to_string();
        novel.set_content(content);
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

    #[test]
    fn test_load_novels_from_dir_filters_extensions() {
        let dir = tempdir().unwrap();
        let txt_path = dir.path().join("book_a.txt");
        let md_path = dir.path().join("note.md");
        let sub_dir = dir.path().join("nested");
        std::fs::write(&txt_path, "hello").unwrap();
        std::fs::write(&md_path, "ignore").unwrap();
        std::fs::create_dir_all(&sub_dir).unwrap();
        std::fs::write(sub_dir.join("book_b.txt"), "nested").unwrap();

        let novels = App::load_novels_from_dir(dir.path()).unwrap();

        assert_eq!(novels.len(), 1);
        assert_eq!(novels[0].title, "book_a");
        assert_eq!(novels[0].path, txt_path);
    }

    #[test]
    fn test_load_novels_from_dir_sorts_by_title() {
        let dir = tempdir().unwrap();
        let z_path = dir.path().join("Zoo.txt");
        let a_path = dir.path().join("apple.txt");
        let b_path = dir.path().join("Book.txt");
        std::fs::write(&z_path, "z").unwrap();
        std::fs::write(&a_path, "a").unwrap();
        std::fs::write(&b_path, "b").unwrap();

        let novels = App::load_novels_from_dir(dir.path()).unwrap();

        assert_eq!(novels.len(), 3);
        assert_eq!(novels[0].path, a_path);
        assert_eq!(novels[1].path, b_path);
        assert_eq!(novels[2].path, z_path);
    }

    #[test]
    fn test_detect_orphaned_novels_collects_missing_and_resets_index() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("exists.txt");
        std::fs::write(&existing, "ok").unwrap();
        let missing = dir.path().join("missing.txt");

        let mut app = create_test_app();
        app.settings.selected_orphaned_index = Some(3);
        app.library.novels = vec![
            NovelInfo {
                title: "exists".to_string(),
                path: existing,
                progress: ReadingProgress::default(),
            },
            NovelInfo {
                title: "missing".to_string(),
                path: missing.clone(),
                progress: ReadingProgress::default(),
            },
        ];

        app.detect_orphaned_novels();

        assert_eq!(app.settings.orphaned_novels.len(), 1);
        assert_eq!(app.settings.orphaned_novels[0].path, missing);
        assert_eq!(app.settings.selected_orphaned_index, None);
    }

    #[test]
    fn test_delete_novel_removes_file_and_updates_selection() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("first.txt");
        let second = dir.path().join("second.txt");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();

        let mut app = create_test_app();
        app.novels = vec![Novel::new(first.clone()), Novel::new(second.clone())];
        app.library.novels = vec![
            NovelInfo {
                title: "first".to_string(),
                path: first.clone(),
                progress: ReadingProgress::default(),
            },
            NovelInfo {
                title: "second".to_string(),
                path: second.clone(),
                progress: ReadingProgress::default(),
            },
        ];
        app.settings.selected_delete_novel_index = Some(0);

        app.delete_novel(0).unwrap();

        assert!(!first.exists());
        assert_eq!(app.novels.len(), 1);
        assert_eq!(app.novels[0].path, second);
        assert_eq!(app.library.novels.len(), 1);
        assert_eq!(app.settings.selected_delete_novel_index, Some(0));
    }

    #[test]
    fn test_find_chapter_index_boundaries() {
        let chapters = vec![
            Chapter {
                title: "A".to_string(),
                start_line: 10,
            },
            Chapter {
                title: "B".to_string(),
                start_line: 20,
            },
            Chapter {
                title: "C".to_string(),
                start_line: 30,
            },
        ];

        assert_eq!(App::find_chapter_index(&chapters, 0), 0);
        assert_eq!(App::find_chapter_index(&chapters, 20), 1);
        assert_eq!(App::find_chapter_index(&chapters, 999), 2);
    }

    #[test]
    fn test_poll_sync_status_handles_progress_and_upload_complete() {
        let mut app = create_test_app();
        let (tx, rx) = mpsc::channel();
        app.sync_rx = Some(rx);

        tx.send(SyncMessage::Progress("进行中".to_string()))
            .unwrap();
        tx.send(SyncMessage::UploadComplete).unwrap();

        app.poll_sync_status();

        assert_eq!(app.sync_status, SyncStatus::Success("上传完成".to_string()));
        assert!(app.sync_rx.is_none());
    }

    #[test]
    fn test_poll_sync_status_handles_failed() {
        let mut app = create_test_app();
        let (tx, rx) = mpsc::channel();
        app.sync_rx = Some(rx);

        tx.send(SyncMessage::Failed("网络错误".to_string()))
            .unwrap();

        app.poll_sync_status();

        assert_eq!(app.sync_status, SyncStatus::Error("网络错误".to_string()));
        assert!(app.sync_rx.is_none());
    }

    #[test]
    fn test_trigger_sync_requires_webdav_config() {
        let mut app = create_test_app();
        app.webdav_config.enabled = false;

        app.trigger_sync();

        assert_eq!(app.error_message.as_deref(), Some("请先配置 WebDAV"));
        assert!(app.sync_rx.is_none());
    }

    #[test]
    fn test_trigger_download_requires_webdav_config() {
        let mut app = create_test_app();
        app.webdav_config.enabled = false;

        app.trigger_download();

        assert_eq!(app.error_message.as_deref(), Some("请先配置 WebDAV"));
        assert!(app.sync_rx.is_none());
    }

    #[test]
    fn test_delete_novel_out_of_bounds_is_noop() {
        let mut app = create_test_app();
        app.novels = vec![Novel::new(PathBuf::from("first.txt"))];
        app.library.novels = vec![NovelInfo {
            title: "first".to_string(),
            path: PathBuf::from("first.txt"),
            progress: ReadingProgress::default(),
        }];
        app.settings.selected_delete_novel_index = Some(0);

        let result = app.delete_novel(99);

        assert!(result.is_ok());
        assert_eq!(app.novels.len(), 1);
        assert_eq!(app.library.novels.len(), 1);
        assert_eq!(app.settings.selected_delete_novel_index, Some(0));
    }
}
