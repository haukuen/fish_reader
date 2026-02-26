use crate::app::App;
use crate::state::AppState;
use crossterm::event::KeyCode;

use super::count_physical_lines;

/// 处理阅读器模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `q`: 退出应用（保存进度）
/// - `Esc`: 返回书架（保存进度）
/// - `Up`/`k`: 向上滚动一行
/// - `Down`/`j`: 向下滚动一行
/// - `Left`/`h`: 向上翻页
/// - `Right`/`l`: 向下翻页
/// - `/`: 进入搜索模式
/// - `t`: 进入章节目录
/// - `b`: 进入书签列表
/// - `m`: 添加书签
/// - `[`: 跳转到上一章
/// - `]`: 跳转到下一章
pub(super) fn handle_reader_key(app: &mut App, key: KeyCode) {
    if let Some(novel) = &mut app.current_novel {
        let max_scroll = novel.line_count().saturating_sub(1);

        let content_width = app.terminal_size.width.saturating_sub(4) as usize;
        let content_height = (app.terminal_size.height as usize)
            .saturating_sub(1)
            .saturating_sub(2)
            .saturating_sub(1);
        let page_size = content_height.max(1);

        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.save_current_progress();
                app.should_quit = true;
            }
            KeyCode::Esc => {
                app.save_current_progress();
                app.state = AppState::Bookshelf;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if novel.progress.scroll_offset > 0 {
                    novel.progress.scroll_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if novel.progress.scroll_offset < max_scroll.saturating_sub(page_size) {
                    novel.progress.scroll_offset += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let mut physical_lines_in_prev_page = 0;
                let mut logical_lines_to_jump = 0;

                for line in novel
                    .lines()
                    .iter()
                    .take(novel.progress.scroll_offset)
                    .rev()
                {
                    let line_height = count_physical_lines(line, content_width);
                    if physical_lines_in_prev_page + line_height > page_size {
                        break;
                    }
                    physical_lines_in_prev_page += line_height;
                    logical_lines_to_jump += 1;
                }

                novel.progress.scroll_offset = novel
                    .progress
                    .scroll_offset
                    .saturating_sub(logical_lines_to_jump.max(1));
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let mut physical_lines_on_current_page = 0;
                let mut logical_lines_to_jump = 0;

                for line in novel.lines().iter().skip(novel.progress.scroll_offset) {
                    let line_height = count_physical_lines(line, content_width);
                    if physical_lines_on_current_page + line_height > page_size {
                        break;
                    }
                    physical_lines_on_current_page += line_height;
                    logical_lines_to_jump += 1;
                }

                let jump = logical_lines_to_jump.max(1);
                novel.progress.scroll_offset =
                    (novel.progress.scroll_offset + jump).min(max_scroll);
            }
            KeyCode::Char('/') => {
                app.previous_state = AppState::Reading;
                app.state = AppState::Searching;
                app.search.clear();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.previous_state = AppState::Reading;
                app.state = AppState::ChapterList;
                app.selected_chapter_index = app.find_current_chapter_index();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.previous_state = AppState::Reading;
                app.state = AppState::BookmarkList;
                app.bookmark.selected_index = None;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                app.previous_state = AppState::Reading;
                app.state = AppState::BookmarkAdd;
                app.clear_bookmark_inputs();
            }
            KeyCode::Char('[') => {
                if !novel.chapters.is_empty() {
                    let current_idx =
                        App::find_chapter_index(&novel.chapters, novel.progress.scroll_offset);
                    if current_idx > 0 {
                        novel.progress.scroll_offset = novel.chapters[current_idx - 1].start_line;
                        app.save_current_progress();
                    }
                }
            }
            KeyCode::Char(']') => {
                if !novel.chapters.is_empty() {
                    let current_idx =
                        App::find_chapter_index(&novel.chapters, novel.progress.scroll_offset);
                    if current_idx + 1 < novel.chapters.len() {
                        novel.progress.scroll_offset = novel.chapters[current_idx + 1].start_line;
                        app.save_current_progress();
                    }
                }
            }
            _ => {}
        }
    }
}
