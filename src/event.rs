use crate::app::App;
use crate::state::AppState;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use unicode_width::UnicodeWidthStr;

/// 估算一个字符串在换行时将占据的物理行数。
fn count_physical_lines(line: &str, width: usize) -> usize {
    if line.is_empty() {
        return 1; // 空行也会占用一个物理行
    }
    if width == 0 {
        return 1;
    }
    // 计算字符串的总宽度，并除以可用宽度。
    // 向上取整以得到行数。
    line.width().div_ceil(width)
}

pub fn handle_key(app: &mut App, key: KeyCode) {
    match app.state {
        AppState::Bookshelf => handle_bookshelf_key(app, key),
        AppState::Reading => handle_reader_key(app, key),
        AppState::Searching => handle_search_key(app, key),
        AppState::ChapterList => handle_chapter_list_key(app, key),
        AppState::Settings => handle_settings_key(app, key),
        AppState::BookmarkList => handle_bookmark_list_key(app, key),
        AppState::BookmarkAdd => handle_bookmark_add_key(app, key),
    }
}

/// 处理书签列表模式下的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 处理书签选择、跳转和删除
fn handle_bookmark_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回上一个状态
            app.state = app.previous_state.clone();
        }
        KeyCode::Enter => {
            if let Some(index) = app.bookmark.selected_index {
                // 跳转到选中的书签
                if app.jump_to_bookmark(index).is_some() {
                    app.state = AppState::Reading;
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(bookmarks) = app.get_current_bookmarks()
                && !bookmarks.is_empty()
            {
                let current = app.bookmark.selected_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    bookmarks.len() - 1
                };
                app.bookmark.selected_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(bookmarks) = app.get_current_bookmarks()
                && !bookmarks.is_empty()
            {
                if app.bookmark.selected_index.is_none() {
                    app.bookmark.selected_index = Some(0);
                } else {
                    let current = app.bookmark.selected_index.unwrap();
                    let next = (current + 1) % bookmarks.len();
                    app.bookmark.selected_index = Some(next);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // 删除选中的书签
            if let Some(index) = app.bookmark.selected_index
                && app.remove_bookmark(index).is_some()
            {
                // 调整选中索引
                if let Some(bookmarks) = app.get_current_bookmarks() {
                    if !bookmarks.is_empty() {
                        let new_index = index.min(bookmarks.len() - 1);
                        app.bookmark.selected_index = Some(new_index);
                    } else {
                        app.bookmark.selected_index = None;
                    }
                }
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            // 进入添加书签模式
            app.state = AppState::BookmarkAdd;
            app.clear_bookmark_inputs();
        }
        _ => {}
    }
}

/// 处理添加书签模式下的键盘事件
/// # 参数
/// - `key`: 按下的键位代码
/// # 功能
/// 处理书签名称和描述输入
fn handle_bookmark_add_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 取消添加，返回上一个状态
            app.state = app.previous_state.clone();
            app.clear_bookmark_inputs();
        }
        KeyCode::Enter => {
            // 确认添加书签
            if !app.bookmark.input.trim().is_empty() {
                app.add_bookmark(app.bookmark.input.clone());
                app.state = AppState::BookmarkList;
                app.clear_bookmark_inputs();
            }
        }
        KeyCode::Backspace => {
            // 删除输入的最后一个字符
            app.bookmark.input.pop();
        }
        KeyCode::Char(c) => {
            // 添加字符到书签名称输入框
            app.bookmark.input.push(c);
        }
        _ => {}
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
            AppState::BookmarkList => handle_bookmark_list_key(app, KeyCode::Up),
            AppState::BookmarkAdd => {} // 添加书签模式不处理滚动
        },
        MouseEventKind::ScrollDown => match app.state {
            AppState::Reading => handle_reader_key(app, KeyCode::Down),
            AppState::Bookshelf => handle_bookshelf_key(app, KeyCode::Down),
            AppState::ChapterList => handle_chapter_list_key(app, KeyCode::Down),
            AppState::Settings => handle_settings_key(app, KeyCode::Down),
            AppState::Searching => handle_search_key(app, KeyCode::Down),
            AppState::BookmarkList => handle_bookmark_list_key(app, KeyCode::Down),
            AppState::BookmarkAdd => {} // 添加书签模式不处理滚动
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
            if let Some(index) = app.selected_novel_index
                && index < app.novels.len()
            {
                let mut novel = app.novels[index].clone();

                // 懒加载：如果内容为空，加载文件内容
                if novel.content.is_empty() {
                    let _ = novel.load_content();
                }

                // 恢复阅读进度
                novel.progress = app.library.get_novel_progress(&novel.path);

                app.current_novel = Some(novel);
                app.state = AppState::Reading;
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

        let content_width = app.terminal_size.width.saturating_sub(4) as usize;
        let content_height = (app.terminal_size.height as usize)
            .saturating_sub(1) // 帮助信息1行
            .saturating_sub(2) // 上下边框各占1行
            .saturating_sub(1);
        let page_size = content_height.max(1);

        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                // 保存进度
                app.library
                    .update_novel_progress(&novel.path, novel.progress.clone());
                let _ = app.library.save();
                app.should_quit = true;
            }
            KeyCode::Esc => {
                // 保存阅读进度并返回书架
                app.library
                    .update_novel_progress(&novel.path, novel.progress.clone());
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
                let mut physical_lines_in_prev_page = 0;
                let mut logical_lines_to_jump = 0;

                // 从当前行向后迭代以找到前一页的开头
                for line in lines.iter().take(novel.progress.scroll_offset).rev() {
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
                // 向下翻页
                let mut physical_lines_on_current_page = 0;
                let mut logical_lines_to_jump = 0;

                for line in lines.iter().skip(novel.progress.scroll_offset) {
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
                // 进入搜索模式
                app.previous_state = AppState::Reading;
                app.state = AppState::Searching;
                app.search.clear();
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // 进入章节目录模式
                app.previous_state = AppState::Reading;
                app.state = AppState::ChapterList;
                // 根据当前阅读位置自动选择最接近的章节
                app.selected_chapter_index = app.find_current_chapter_index();
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                // 进入书签列表模式
                app.previous_state = AppState::Reading;
                app.state = AppState::BookmarkList;
                app.bookmark.selected_index = None;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                // 进入添加书签模式
                app.previous_state = AppState::Reading;
                app.state = AppState::BookmarkAdd;
                app.clear_bookmark_inputs();
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
            if let Some(index) = app.search.selected_index {
                // 跳转到选中的搜索结果
                if index < app.search.results.len() {
                    let (line_num, _) = app.search.results[index];
                    if let Some(novel) = &mut app.current_novel {
                        novel.progress.scroll_offset = line_num;
                        // 保存进度
                        app.library
                            .update_novel_progress(&novel.path, novel.progress.clone());
                        let _ = app.library.save();
                    }
                    app.state = AppState::Reading;
                }
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
                if app.search.selected_index.is_none() {
                    app.search.selected_index = Some(0);
                } else {
                    let current = app.search.selected_index.unwrap();
                    let next = (current + 1) % app.search.results.len();
                    app.search.selected_index = Some(next);
                }
            }
        }
        KeyCode::Backspace => {
            // 删除搜索输入的最后一个字符
            app.search.input.pop();
            app.perform_search();
        }
        KeyCode::Char(c) => {
            // 添加字符到搜索输入
            app.search.input.push(c);
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
            if let Some(index) = app.selected_chapter_index
                && let Some(novel) = &mut app.current_novel
                && index < novel.chapters.len()
            {
                // 跳转到选中的章节
                let chapter = &novel.chapters[index];
                novel.progress.scroll_offset = chapter.start_line;
                // 保存进度
                app.library
                    .update_novel_progress(&novel.path, novel.progress.clone());
                let _ = app.library.save();
                app.state = AppState::Reading;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(novel) = &app.current_novel
                && !novel.chapters.is_empty()
            {
                let current = app.selected_chapter_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    novel.chapters.len() - 1
                };
                app.selected_chapter_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(novel) = &app.current_novel
                && !novel.chapters.is_empty()
            {
                if app.selected_chapter_index.is_none() {
                    app.selected_chapter_index = Some(0);
                } else {
                    let current = app.selected_chapter_index.unwrap();
                    let next = (current + 1) % novel.chapters.len();
                    app.selected_chapter_index = Some(next);
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
/// 处理设置界面的二级菜单导航和操作
fn handle_settings_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match app.settings.mode {
        SettingsMode::MainMenu => handle_settings_main_menu_key(app, key),
        SettingsMode::DeleteNovel => handle_delete_novel_key(app, key),
        SettingsMode::DeleteOrphaned => handle_delete_orphaned_key(app, key),
    }
}

/// 处理设置主菜单的键盘事件
fn handle_settings_main_menu_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回书架
            app.state = AppState::Bookshelf;
            app.settings.reset();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let menu_count = 2; // 删除小说、清理孤立记录
            if menu_count > 0 {
                let current = app.settings.selected_option.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    menu_count - 1
                };
                app.settings.selected_option = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let menu_count = 2; // 删除小说、清理孤立记录
            if menu_count > 0 {
                if app.settings.selected_option.is_none() {
                    app.settings.selected_option = Some(0);
                } else {
                    let current = app.settings.selected_option.unwrap();
                    let next = (current + 1) % menu_count;
                    app.settings.selected_option = Some(next);
                }
            }
        }
        KeyCode::Enter => {
            if let Some(index) = app.settings.selected_option {
                match index {
                    0 => {
                        // 进入删除小说模式
                        app.settings.mode = SettingsMode::DeleteNovel;
                        app.settings.selected_delete_novel_index = if !app.novels.is_empty() {
                            Some(0)
                        } else {
                            None
                        };
                    }
                    1 => {
                        // 进入删除孤立记录模式
                        app.settings.mode = SettingsMode::DeleteOrphaned;
                        app.detect_orphaned_novels();
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// 处理删除小说模式的键盘事件
fn handle_delete_novel_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回设置主菜单
            app.settings.mode = SettingsMode::MainMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.novels.is_empty() {
                let current = app.settings.selected_delete_novel_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.novels.len() - 1
                };
                app.settings.selected_delete_novel_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.novels.is_empty() {
                if app.settings.selected_delete_novel_index.is_none() {
                    app.settings.selected_delete_novel_index = Some(0);
                } else {
                    let current = app.settings.selected_delete_novel_index.unwrap();
                    let next = (current + 1) % app.novels.len();
                    app.settings.selected_delete_novel_index = Some(next);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // 删除选中的小说
            if let Some(index) = app.settings.selected_delete_novel_index
                && index < app.novels.len()
            {
                let _ = app.delete_novel(index);
            }
        }
        _ => {}
    }
}

/// 处理删除孤立记录模式的键盘事件
fn handle_delete_orphaned_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            // 返回设置主菜单
            app.settings.mode = SettingsMode::MainMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.settings.orphaned_novels.is_empty() {
                let current = app.settings.selected_orphaned_index.unwrap_or(0);
                let next = if current > 0 {
                    current - 1
                } else {
                    app.settings.orphaned_novels.len() - 1
                };
                app.settings.selected_orphaned_index = Some(next);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.settings.orphaned_novels.is_empty() {
                if app.settings.selected_orphaned_index.is_none() {
                    app.settings.selected_orphaned_index = Some(0);
                } else {
                    let current = app.settings.selected_orphaned_index.unwrap();
                    let next = (current + 1) % app.settings.orphaned_novels.len();
                    app.settings.selected_orphaned_index = Some(next);
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // 删除选中的孤立记录
            if let Some(index) = app.settings.selected_orphaned_index
                && index < app.settings.orphaned_novels.len()
            {
                let orphaned_novel = &app.settings.orphaned_novels[index];
                app.library.novels.retain(|n| n.path != orphaned_novel.path);
                let _ = app.library.save();
                app.detect_orphaned_novels();

                // 调整选中索引
                if !app.settings.orphaned_novels.is_empty() {
                    let new_index = index.min(app.settings.orphaned_novels.len() - 1);
                    app.settings.selected_orphaned_index = Some(new_index);
                }
            }
        }
        _ => {}
    }
}
