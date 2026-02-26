use crate::app::App;
use crate::state::AppState;
use crossterm::event::KeyCode;

use super::navigate_list;

/// 处理书架模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Enter`: 打开选中的小说
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `s`: 进入设置页面
pub(super) fn handle_bookshelf_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected_novel_index =
                navigate_list(app.selected_novel_index, app.novels.len(), true);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected_novel_index =
                navigate_list(app.selected_novel_index, app.novels.len(), false);
        }
        KeyCode::Enter => {
            if let Some(index) = app.selected_novel_index
                && index < app.novels.len()
            {
                let mut novel = app.novels[index].clone();

                if novel.is_empty()
                    && let Err(e) = novel.load_content()
                {
                    app.set_error(format!("Failed to load novel: {}", e));
                    return;
                }

                novel.progress = app.library.get_novel_progress(&novel.path);

                app.current_novel = Some(novel);
                app.state = AppState::Reading;
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.settings.reset();
            app.detect_orphaned_novels();
            app.state = AppState::Settings;
        }
        KeyCode::Char('w') | KeyCode::Char('W') => {
            app.trigger_sync();
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.trigger_download();
        }
        _ => {}
    }
}
