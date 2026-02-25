use crate::app::App;
use crate::state::AppState;
use crate::ui::sync_status::SyncStatus;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use unicode_width::UnicodeWidthStr;

mod bookmark;
mod bookshelf;
mod chapter_list;
mod reader;
mod search;
mod settings;

/// 计算字符串在指定宽度下占用的物理行数
///
/// # Arguments
///
/// * `line` - 要计算的字符串
/// * `width` - 可用宽度（字符数）
///
/// # Returns
///
/// 占用的物理行数。空字符串或零宽度返回 1。
pub(super) fn count_physical_lines(line: &str, width: usize) -> usize {
    if line.is_empty() {
        return 1;
    }
    if width == 0 {
        return 1;
    }
    line.width().div_ceil(width)
}

/// 通用列表导航函数
///
/// 根据移动方向计算新的选中索引，支持循环导航。
///
/// # Arguments
///
/// * `current` - 当前选中索引
/// * `len` - 列表长度
/// * `move_up` - 是否向上移动（`true` 为向上，`false` 为向下）
///
/// # Returns
///
/// 新的选中索引。如果列表为空则返回 `None`。
pub(super) fn navigate_list(current: Option<usize>, len: usize, move_up: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }

    let new_idx = if move_up {
        current.map(|idx| idx.saturating_sub(1)).unwrap_or(len - 1)
    } else {
        current.map(|idx| (idx + 1) % len).unwrap_or(0)
    };

    Some(new_idx)
}

/// 处理键盘事件
///
/// 根据当前应用状态将键盘事件分发到对应的处理函数。
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
pub fn handle_key(app: &mut App, key: KeyCode) {
    app.error_message = None;
    if matches!(
        app.sync_status,
        SyncStatus::Success(_) | SyncStatus::Error(_)
    ) {
        app.sync_status = SyncStatus::Idle;
    }

    match app.state {
        AppState::Bookshelf => bookshelf::handle_bookshelf_key(app, key),
        AppState::Reading => reader::handle_reader_key(app, key),
        AppState::Searching => search::handle_search_key(app, key),
        AppState::ChapterList => chapter_list::handle_chapter_list_key(app, key),
        AppState::Settings => settings::handle_settings_key(app, key),
        AppState::BookmarkList => bookmark::handle_bookmark_list_key(app, key),
        AppState::BookmarkAdd => bookmark::handle_bookmark_add_key(app, key),
    }
}

/// 处理鼠标事件
///
/// 将鼠标滚动事件转换为对应的键盘事件并分发。
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `mouse` - 鼠标事件
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => match app.state {
            AppState::Reading => reader::handle_reader_key(app, KeyCode::Up),
            AppState::Bookshelf => bookshelf::handle_bookshelf_key(app, KeyCode::Up),
            AppState::ChapterList => chapter_list::handle_chapter_list_key(app, KeyCode::Up),
            AppState::Settings => settings::handle_settings_key(app, KeyCode::Up),
            AppState::Searching => search::handle_search_key(app, KeyCode::Up),
            AppState::BookmarkList => bookmark::handle_bookmark_list_key(app, KeyCode::Up),
            AppState::BookmarkAdd => {}
        },
        MouseEventKind::ScrollDown => match app.state {
            AppState::Reading => reader::handle_reader_key(app, KeyCode::Down),
            AppState::Bookshelf => bookshelf::handle_bookshelf_key(app, KeyCode::Down),
            AppState::ChapterList => chapter_list::handle_chapter_list_key(app, KeyCode::Down),
            AppState::Settings => settings::handle_settings_key(app, KeyCode::Down),
            AppState::Searching => search::handle_search_key(app, KeyCode::Down),
            AppState::BookmarkList => bookmark::handle_bookmark_list_key(app, KeyCode::Down),
            AppState::BookmarkAdd => {}
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, BookmarkState, SearchState, SettingsState};
    use crate::model::library::Library;
    use crate::model::novel::Novel;
    use crate::state::AppState;
    use crate::sync::config::WebDavConfig;
    use crossterm::event::{KeyModifiers, MouseEvent};
    use ratatui::layout::Rect;
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
            webdav_config: WebDavConfig::default(),
            sync_rx: None,
            sync_status: SyncStatus::Idle,
        }
    }

    #[test]
    fn test_count_physical_lines_empty() {
        assert_eq!(count_physical_lines("", 80), 1);
    }

    #[test]
    fn test_count_physical_lines_zero_width() {
        assert_eq!(count_physical_lines("hello", 0), 1);
    }

    #[test]
    fn test_count_physical_lines_single_line() {
        assert_eq!(count_physical_lines("hello", 80), 1);
    }

    #[test]
    fn test_count_physical_lines_wrap() {
        assert_eq!(count_physical_lines("1234567890", 4), 3);
        assert_eq!(count_physical_lines("12345678", 4), 2);
    }

    #[test]
    fn test_count_physical_lines_chinese() {
        assert_eq!(count_physical_lines("你好", 4), 1);
        assert_eq!(count_physical_lines("你好", 3), 2);
        assert_eq!(count_physical_lines("你好世界", 4), 2);
    }

    #[test]
    fn test_count_physical_lines_exact_fit() {
        assert_eq!(count_physical_lines("1234", 4), 1);
        assert_eq!(count_physical_lines("12345", 5), 1);
    }

    #[test]
    fn test_navigate_list_wrap_and_empty() {
        assert_eq!(navigate_list(None, 3, false), Some(0));
        assert_eq!(navigate_list(Some(0), 3, true), Some(0));
        assert_eq!(navigate_list(Some(2), 3, false), Some(0));
        assert_eq!(navigate_list(Some(0), 0, false), None);
    }

    #[test]
    fn test_handle_key_bookshelf_quit() {
        let mut app = create_test_app();
        app.state = AppState::Bookshelf;

        handle_key(&mut app, KeyCode::Char('q'));

        assert!(app.should_quit);
    }

    #[test]
    fn test_handle_key_search_enter_jump_to_reading() {
        let mut app = create_test_app();
        let novel = Novel::new(PathBuf::from("test.txt"));
        app.current_novel = Some(novel);
        app.state = AppState::Searching;
        app.search.results = vec![(7, "line".to_string())];
        app.search.selected_index = Some(0);

        handle_key(&mut app, KeyCode::Enter);

        assert!(app.state == AppState::Reading);
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            7
        );
    }

    #[test]
    fn test_handle_mouse_scroll_down_bookshelf_changes_selection() {
        let mut app = create_test_app();
        app.state = AppState::Bookshelf;
        app.novels = vec![
            Novel::new(PathBuf::from("a.txt")),
            Novel::new(PathBuf::from("b.txt")),
        ];

        let mouse_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, mouse_down);
        handle_mouse(&mut app, mouse_down);

        assert_eq!(app.selected_novel_index, Some(1));
    }

    #[test]
    fn test_handle_mouse_ignored_in_bookmark_add() {
        let mut app = create_test_app();
        app.state = AppState::BookmarkAdd;
        app.bookmark.input = "abc".to_string();

        let mouse_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse(&mut app, mouse_up);

        assert_eq!(app.bookmark.input, "abc");
        assert!(app.state == AppState::BookmarkAdd);
    }
}
