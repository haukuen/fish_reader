use crate::app::App;
use crate::state::AppState;
use crossterm::event::KeyCode;

use super::navigate_list;

/// 处理书签列表模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回上一个状态
/// - `Enter`: 跳转到选中的书签
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `d`: 删除选中的书签
/// - `a`: 进入添加书签模式
pub(super) fn handle_bookmark_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = app.previous_state.clone();
        }
        KeyCode::Enter => {
            if let Some(index) = app.bookmark.selected_index
                && app.jump_to_bookmark(index).is_some()
            {
                app.state = AppState::Reading;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(bookmarks) = app.get_current_bookmarks() {
                app.bookmark.selected_index =
                    navigate_list(app.bookmark.selected_index, bookmarks.len(), true);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(bookmarks) = app.get_current_bookmarks() {
                app.bookmark.selected_index =
                    navigate_list(app.bookmark.selected_index, bookmarks.len(), false);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(index) = app.bookmark.selected_index
                && app.remove_bookmark(index).is_some()
                && let Some(bookmarks) = app.get_current_bookmarks()
            {
                app.bookmark.selected_index = if bookmarks.is_empty() {
                    None
                } else {
                    Some(index.min(bookmarks.len() - 1))
                };
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.state = AppState::BookmarkAdd;
            app.clear_bookmark_inputs();
        }
        _ => {}
    }
}

/// 处理添加书签模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 取消添加，返回上一个状态
/// - `Enter`: 确认添加书签
/// - `Backspace`: 删除输入的最后一个字符
/// - 其他字符: 添加到输入框
pub(super) fn handle_bookmark_add_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = app.previous_state.clone();
            app.clear_bookmark_inputs();
        }
        KeyCode::Enter => {
            if !app.bookmark.input.trim().is_empty() {
                app.add_bookmark(app.bookmark.input.clone());
                app.state = AppState::BookmarkList;
                app.clear_bookmark_inputs();
            }
        }
        KeyCode::Backspace => {
            app.bookmark.input.pop();
        }
        KeyCode::Char(c) => {
            app.bookmark.input.push(c);
        }
        _ => {}
    }
}
