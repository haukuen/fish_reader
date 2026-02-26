use crate::app::App;
use crate::state::AppState;
use crossterm::event::KeyCode;

/// 处理搜索模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Enter`: 跳转到选中的搜索结果
/// - `Up`: 向上选择搜索结果
/// - `Down`: 向下选择搜索结果
/// - `Backspace`: 删除输入的最后一个字符
/// - 其他字符: 添加到搜索框并执行搜索
pub(super) fn handle_search_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            if let Some(index) = app.search.selected_index
                && index < app.search.results.len()
            {
                let (line_num, _) = app.search.results[index];
                if let Some(novel) = &mut app.current_novel {
                    novel.progress.scroll_offset = line_num;
                    app.save_current_progress();
                }
                app.state = AppState::Reading;
            }
        }
        KeyCode::Up => {
            if !app.search.results.is_empty() {
                let current = app.search.selected_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.search.results.len() - 1
                };
                app.search.selected_index = Some(next);
            }
        }
        KeyCode::Down => {
            if !app.search.results.is_empty() {
                let next = match app.search.selected_index {
                    None => 0,
                    Some(current) => (current + 1) % app.search.results.len(),
                };
                app.search.selected_index = Some(next);
            }
        }
        KeyCode::Backspace => {
            app.search.input.pop();
            app.perform_search();
        }
        KeyCode::Char(c) => {
            app.search.input.push(c);
            app.perform_search();
        }
        _ => {}
    }
}
