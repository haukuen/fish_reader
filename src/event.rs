use crate::app::App;
use crate::state::AppState;
use crate::ui::sync_status::SyncStatus;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use unicode_width::UnicodeWidthStr;

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
fn count_physical_lines(line: &str, width: usize) -> usize {
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
fn navigate_list(current: Option<usize>, len: usize, move_up: bool) -> Option<usize> {
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
fn handle_bookmark_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = app.previous_state.clone();
        }
        KeyCode::Enter => {
            if let Some(index) = app.bookmark.selected_index {
                if app.jump_to_bookmark(index).is_some() {
                    app.state = AppState::Reading;
                }
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
            {
                if let Some(bookmarks) = app.get_current_bookmarks() {
                    app.bookmark.selected_index = if bookmarks.is_empty() {
                        None
                    } else {
                        Some(index.min(bookmarks.len() - 1))
                    };
                }
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
fn handle_bookmark_add_key(app: &mut App, key: KeyCode) {
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
            AppState::Reading => handle_reader_key(app, KeyCode::Up),
            AppState::Bookshelf => handle_bookshelf_key(app, KeyCode::Up),
            AppState::ChapterList => handle_chapter_list_key(app, KeyCode::Up),
            AppState::Settings => handle_settings_key(app, KeyCode::Up),
            AppState::Searching => handle_search_key(app, KeyCode::Up),
            AppState::BookmarkList => handle_bookmark_list_key(app, KeyCode::Up),
            AppState::BookmarkAdd => {}
        },
        MouseEventKind::ScrollDown => match app.state {
            AppState::Reading => handle_reader_key(app, KeyCode::Down),
            AppState::Bookshelf => handle_bookshelf_key(app, KeyCode::Down),
            AppState::ChapterList => handle_chapter_list_key(app, KeyCode::Down),
            AppState::Settings => handle_settings_key(app, KeyCode::Down),
            AppState::Searching => handle_search_key(app, KeyCode::Down),
            AppState::BookmarkList => handle_bookmark_list_key(app, KeyCode::Down),
            AppState::BookmarkAdd => {}
        },
        _ => {}
    }
}

/// 处理书架模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 退出应用
/// - `Enter`: 打开选中的小说
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `s`: 进入设置页面
fn handle_bookshelf_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.should_quit = true;
        }
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
fn handle_reader_key(app: &mut App, key: KeyCode) {
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

/// 处理搜索模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回上一个状态
/// - `Enter`: 跳转到选中的搜索结果
/// - `Up`: 向上选择搜索结果
/// - `Down`: 向下选择搜索结果
/// - `Backspace`: 删除输入的最后一个字符
/// - 其他字符: 添加到搜索框并执行搜索
fn handle_search_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = app.previous_state.clone();
        }
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

/// 处理章节目录模式下的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回阅读模式
/// - `Enter`: 跳转到选中的章节
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
fn handle_chapter_list_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = app.previous_state.clone();
        }
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

/// 处理设置页面的键盘事件
///
/// 根据当前设置模式分发到对应的处理函数。
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
fn handle_settings_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match app.settings.mode {
        SettingsMode::MainMenu => handle_settings_main_menu_key(app, key),
        SettingsMode::DeleteNovel => handle_delete_novel_key(app, key),
        SettingsMode::DeleteOrphaned => handle_delete_orphaned_key(app, key),
        SettingsMode::WebDavConfig => handle_webdav_config_key(app, key),
    }
}

/// 处理设置主菜单的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回书架
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `Enter`: 进入选中的子菜单
fn handle_settings_main_menu_key(app: &mut App, key: KeyCode) {
    use crate::config::CONFIG;
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.state = AppState::Bookshelf;
            app.settings.reset();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings.selected_option = navigate_list(
                app.settings.selected_option,
                CONFIG.settings_menu_count,
                true,
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings.selected_option = navigate_list(
                app.settings.selected_option,
                CONFIG.settings_menu_count,
                false,
            );
        }
        KeyCode::Enter => {
            if let Some(index) = app.settings.selected_option {
                match index {
                    0 => {
                        app.settings.mode = SettingsMode::DeleteNovel;
                        app.settings.selected_delete_novel_index = if !app.novels.is_empty() {
                            Some(0)
                        } else {
                            None
                        };
                    }
                    1 => {
                        app.settings.mode = SettingsMode::DeleteOrphaned;
                        app.detect_orphaned_novels();
                    }
                    2 => {
                        app.settings.mode = SettingsMode::WebDavConfig;
                        app.settings.webdav_config_state.temp_config = app.webdav_config.clone();
                        app.settings.webdav_config_state.selected_field = 0;
                        app.settings.webdav_config_state.edit_mode = false;
                        app.settings.webdav_config_state.show_password = false;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// 处理删除小说模式的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回设置主菜单
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `d`: 删除选中的小说
fn handle_delete_novel_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.settings.mode = SettingsMode::MainMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings.selected_delete_novel_index = navigate_list(
                app.settings.selected_delete_novel_index,
                app.novels.len(),
                true,
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings.selected_delete_novel_index = navigate_list(
                app.settings.selected_delete_novel_index,
                app.novels.len(),
                false,
            );
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(index) = app.settings.selected_delete_novel_index
                && index < app.novels.len()
                && let Err(e) = app.delete_novel(index)
            {
                app.set_error(format!("Failed to delete novel: {}", e));
            }
        }
        _ => {}
    }
}

/// 处理删除孤立记录模式的键盘事件
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
///
/// # Behavior
///
/// - `Esc`/`q`: 返回设置主菜单
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `d`: 删除选中的孤立记录
fn handle_delete_orphaned_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;

    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.settings.mode = SettingsMode::MainMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.settings.selected_orphaned_index = navigate_list(
                app.settings.selected_orphaned_index,
                app.settings.orphaned_novels.len(),
                true,
            );
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.settings.selected_orphaned_index = navigate_list(
                app.settings.selected_orphaned_index,
                app.settings.orphaned_novels.len(),
                false,
            );
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(index) = app.settings.selected_orphaned_index
                && index < app.settings.orphaned_novels.len()
            {
                let orphaned_novel = &app.settings.orphaned_novels[index];
                app.library.novels.retain(|n| n.path != orphaned_novel.path);
                if let Err(e) = app.library.save() {
                    app.set_error(format!("Failed to save: {}", e));
                }
                app.detect_orphaned_novels();

                if !app.settings.orphaned_novels.is_empty() {
                    let new_index = index.min(app.settings.orphaned_novels.len() - 1);
                    app.settings.selected_orphaned_index = Some(new_index);
                }
            }
        }
        _ => {}
    }
}

/// 处理WebDAV配置界面的键盘事件
fn handle_webdav_config_key(app: &mut App, key: KeyCode) {
    use crate::state::SettingsMode;
    use crate::sync::webdav_client::WebDavClient;

    let config_state = &mut app.settings.webdav_config_state;

    if config_state.edit_mode {
        match key {
            KeyCode::Esc => {
                config_state.edit_mode = false;
            }
            KeyCode::Enter => {
                config_state.edit_mode = false;
            }
            KeyCode::Backspace => match config_state.selected_field {
                1 => {
                    config_state.temp_config.url.pop();
                }
                2 => {
                    config_state.temp_config.username.pop();
                }
                3 => {
                    config_state.temp_config.password.pop();
                }
                4 => {
                    config_state.temp_config.remote_path.pop();
                }
                _ => {}
            },
            KeyCode::Char(c) => match config_state.selected_field {
                1 => {
                    config_state.temp_config.url.push(c);
                }
                2 => {
                    config_state.temp_config.username.push(c);
                }
                3 => {
                    config_state.temp_config.password.push(c);
                }
                4 => {
                    config_state.temp_config.remote_path.push(c);
                }
                _ => {}
            },
            _ => {}
        }
    } else {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.settings.mode = SettingsMode::MainMenu;
            }
            KeyCode::Up => {
                if config_state.selected_field > 0 {
                    config_state.selected_field -= 1;
                }
            }
            KeyCode::Down => {
                if config_state.selected_field < 4 {
                    config_state.selected_field += 1;
                }
            }
            KeyCode::Tab => {
                if config_state.selected_field == 0 {
                    config_state.temp_config.enabled = !config_state.temp_config.enabled;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                config_state.show_password = !config_state.show_password;
            }
            KeyCode::Enter => match config_state.selected_field {
                0 => {
                    config_state.temp_config.enabled = !config_state.temp_config.enabled;
                }
                1..=4 => {
                    config_state.edit_mode = true;
                }
                _ => {}
            },
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.save_webdav_config();
                app.settings.mode = SettingsMode::MainMenu;
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                let temp_config = &app.settings.webdav_config_state.temp_config;
                let result = match WebDavClient::new(temp_config) {
                    Ok(client) => match client.test_connection(&temp_config.remote_path) {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e.to_string()),
                    },
                    Err(e) => Err(e.to_string()),
                };
                app.settings.webdav_config_state.connection_status = Some(result);
            }
            _ => {}
        }
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
