use crate::app::App;
use crate::config::CONFIG;
use crate::state::SettingsMode;
use crate::sync::webdav_client::WebDavClient;
use crossterm::event::KeyCode;

use super::navigate_list;

/// 处理设置页面的键盘事件
///
/// 根据当前设置模式分发到对应的处理函数。
///
/// # Arguments
///
/// * `app` - 应用实例的可变引用
/// * `key` - 按下的键位代码
pub(super) fn handle_settings_key(app: &mut App, key: KeyCode) {
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
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `Enter`: 进入选中的子菜单
fn handle_settings_main_menu_key(app: &mut App, key: KeyCode) {
    match key {
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
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `d`: 删除选中的小说
fn handle_delete_novel_key(app: &mut App, key: KeyCode) {
    match key {
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
/// - `Up`/`k`: 向上选择
/// - `Down`/`j`: 向下选择
/// - `d`: 删除选中的孤立记录
fn handle_delete_orphaned_key(app: &mut App, key: KeyCode) {
    match key {
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
