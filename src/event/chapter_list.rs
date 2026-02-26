use crate::app::App;
use crate::state::AppState;
use crossterm::event::KeyCode;

use super::navigate_list;

/// 处理章节目录模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Enter`: 跳转到选中的章节
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
pub(super) fn handle_chapter_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            if let Some(index) = app.selected_chapter_index
                && let Some(novel) = &mut app.current_novel
                && index < novel.chapters.len()
            {
                let chapter = &novel.chapters[index];
                novel.progress.scroll_offset = chapter.start_line;
                app.save_current_progress();
                app.state = AppState::Reading;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(novel) = &app.current_novel {
                app.selected_chapter_index =
                    navigate_list(app.selected_chapter_index, novel.chapters.len(), true);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(novel) = &app.current_novel {
                app.selected_chapter_index =
                    navigate_list(app.selected_chapter_index, novel.chapters.len(), false);
            }
        }
        _ => {}
    }
}
