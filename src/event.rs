use crate::app::App;
use crate::state::AppState;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};

pub fn handle_key(app: &mut App, key: KeyCode) {
    match app.state {
        AppState::Bookshelf => handle_bookshelf_key(app, key),
        AppState::Reading => handle_reader_key(app, key),
        AppState::Searching => handle_search_key(app, key),
        AppState::ChapterList => handle_chapter_list_key(app, key),
        AppState::Settings => handle_settings_key(app, key),
    }
}

/// 处理鼠标事件
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => match app.state {
            AppState::Reading => handle_reader_key(app, KeyCode::Up),
            AppState::Bookshelf => handle_bookshelf_key(app, KeyCode::Up),
            AppState::ChapterList => handle_chapter_list_key(app, KeyCode::Up),
            AppState::Settings => handle_settings_key(app, KeyCode::Up),
            AppState::Searching => handle_search_key(app, KeyCode::Up),
        },
        MouseEventKind::ScrollDown => match app.state {
            AppState::Reading => handle_reader_key(app, KeyCode::Down),
            AppState::Bookshelf => handle_bookshelf_key(app, KeyCode::Down),
            AppState::ChapterList => handle_chapter_list_key(app, KeyCode::Down),
            AppState::Settings => handle_settings_key(app, KeyCode::Down),
            AppState::Searching => handle_search_key(app, KeyCode::Down),
        },
        _ => {}
    }
}

fn handle_bookshelf_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.novels.is_empty() {
                let current = app.selected_novel_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.novels.len() - 1
                };
                app.selected_novel_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.novels.is_empty() {
                // 如果当前没有选中任何小说，则选中第一本
                if app.selected_novel_index.is_none() {
                    app.selected_novel_index = Some(0);
                } else {
                    let current = app.selected_novel_index.unwrap();
                    let next = (current + 1) % app.novels.len();
                    app.selected_novel_index = Some(next);
                }
            }
        }
        KeyCode::Enter => {
            if let Some(index) = app.selected_novel_index {
                if index < app.novels.len() {
                    let mut novel = app.novels[index].clone();

                    // 恢复阅读进度
                    novel.progress = app.library.get_novel_progress(&novel.path);

                    app.current_novel = Some(novel);
                    app.state = AppState::Reading;
                }
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // 进入设置页面
            app.detect_orphaned_novels();
            app.state = AppState::Settings;
        }
        _ => {}
    }
}

/// 处理阅读器模式下的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 实现滚动控制、进度保存和界面切换
fn handle_reader_key(app: &mut App, key: KeyCode) {
    if let Some(novel) = &mut app.current_novel {
        let lines: Vec<&str> = novel.content.lines().collect();
        let max_scroll = lines.len().saturating_sub(1);

        let content_height = (app.terminal_size.height as usize)
            .saturating_sub(1) // 帮助信息1行
            .saturating_sub(2) // 上下边框各占1行
            .saturating_sub(1);
        let page_size = content_height.max(1);

        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // 保存阅读进度
                app.library
                    .update_novel_progress(&novel.path, novel.progress);
                let _ = app.library.save();
                app.should_quit = true;
            }
            KeyCode::Esc => {
                // 保存阅读进度并返回书架
                app.library
                    .update_novel_progress(&novel.path, novel.progress);
                let _ = app.library.save();
                app.state = AppState::Bookshelf;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // 向上滚动一行
                if novel.progress.scroll_offset > 0 {
                    novel.progress.scroll_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // 向下滚动一行
                if novel.progress.scroll_offset < max_scroll.saturating_sub(page_size) {
                    novel.progress.scroll_offset += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                // 向上翻页
                novel.progress.scroll_offset =
                    novel.progress.scroll_offset.saturating_sub(page_size);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max_offset = lines.len().saturating_sub(content_height);
                novel.progress.scroll_offset =
                    (novel.progress.scroll_offset + page_size).min(max_offset);
            }
            KeyCode::Char('/') => {
                // 进入搜索模式
                app.previous_state = AppState::Reading;
                app.state = AppState::Searching;
                app.search_input.clear();
                app.search_results.clear();
                app.selected_search_index = None;
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // 进入章节目录模式
                app.previous_state = AppState::Reading;
                app.state = AppState::ChapterList;
                // 根据当前阅读位置自动选择最接近的章节
                app.selected_chapter_index = app.find_current_chapter_index();
            }
            _ => {}
        }
    }
}

/// 处理搜索模式下的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 处理搜索输入、结果选择和跳转
fn handle_search_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回上一个状态
            app.state = app.previous_state.clone();
        }
        KeyCode::Enter => {
            if let Some(index) = app.selected_search_index {
                // 跳转到选中的搜索结果
                if index < app.search_results.len() {
                    let (line_num, _) = app.search_results[index];
                    if let Some(novel) = &mut app.current_novel {
                        novel.progress.scroll_offset = line_num;
                        // 保存进度
                        app.library
                            .update_novel_progress(&novel.path, novel.progress);
                        let _ = app.library.save();
                    }
                    app.state = AppState::Reading;
                }
            }
        }
        KeyCode::Up => {
            if !app.search_results.is_empty() {
                let current = app.selected_search_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.search_results.len() - 1
                };
                app.selected_search_index = Some(next);
            }
        }
        KeyCode::Down => {
            if !app.search_results.is_empty() {
                if app.selected_search_index.is_none() {
                    app.selected_search_index = Some(0);
                } else {
                    let current = app.selected_search_index.unwrap();
                    let next = (current + 1) % app.search_results.len();
                    app.selected_search_index = Some(next);
                }
            }
        }
        KeyCode::Backspace => {
            // 删除搜索输入的最后一个字符
            app.search_input.pop();
            app.perform_search();
        }
        KeyCode::Char(c) => {
            // 添加字符到搜索输入
            app.search_input.push(c);
            app.perform_search();
        }
        _ => {}
    }
}

/// 处理章节目录模式下的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 处理章节选择、跳转和返回
fn handle_chapter_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回阅读模式
            app.state = app.previous_state.clone();
        }
        KeyCode::Enter => {
            if let Some(index) = app.selected_chapter_index {
                if let Some(novel) = &mut app.current_novel {
                    if index < novel.chapters.len() {
                        // 跳转到选中的章节
                        let chapter = &novel.chapters[index];
                        novel.progress.scroll_offset = chapter.start_line;
                        // 保存进度
                        app.library
                            .update_novel_progress(&novel.path, novel.progress);
                        let _ = app.library.save();
                        app.state = AppState::Reading;
                    }
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(novel) = &app.current_novel {
                if !novel.chapters.is_empty() {
                    let current = app.selected_chapter_index.unwrap_or(0);
                    let next = if current > 0 {
                        current - 1
                    } else {
                        novel.chapters.len() - 1
                    };
                    app.selected_chapter_index = Some(next);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(novel) = &app.current_novel {
                if !novel.chapters.is_empty() {
                    if app.selected_chapter_index.is_none() {
                        app.selected_chapter_index = Some(0);
                    } else {
                        let current = app.selected_chapter_index.unwrap();
                        let next = (current + 1) % novel.chapters.len();
                        app.selected_chapter_index = Some(next);
                    }
                }
            }
        }
        _ => {}
    }
}

/// 处理设置页面的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 处理孤立记录的选择和删除操作
fn handle_settings_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回书架
            app.state = AppState::Bookshelf;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.orphaned_novels.is_empty() {
                let current = app.selected_orphaned_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.orphaned_novels.len() - 1
                };
                app.selected_orphaned_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.orphaned_novels.is_empty() {
                if app.selected_orphaned_index.is_none() {
                    app.selected_orphaned_index = Some(0);
                } else {
                    let current = app.selected_orphaned_index.unwrap();
                    let next = (current + 1) % app.orphaned_novels.len();
                    app.selected_orphaned_index = Some(next);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // 删除选中的孤立记录
            if let Some(index) = app.selected_orphaned_index {
                if index < app.orphaned_novels.len() {
                    let orphaned_novel = &app.orphaned_novels[index];

                    app.library.novels.retain(|n| n.path != orphaned_novel.path);

                    let _ = app.library.save();

                    app.detect_orphaned_novels();

                    // 调整选中索引
                    if !app.orphaned_novels.is_empty() {
                        let new_index = index.min(app.orphaned_novels.len() - 1);
                        app.selected_orphaned_index = Some(new_index);
                    }
                }
            }
        }
        _ => {}
    }
}
