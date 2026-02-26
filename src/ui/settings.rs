use ratatui::prelude::*;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;
use crate::state::SettingsMode;

pub fn render_settings(f: &mut Frame, app: &App) {
    let area = f.area();

    match app.settings.mode {
        SettingsMode::MainMenu => render_settings_main_menu(f, app, area),
        SettingsMode::DeleteNovel => render_delete_novel_menu(f, app, area),
        SettingsMode::DeleteOrphaned => render_delete_orphaned_menu(f, app, area),
        SettingsMode::WebDavConfig => render_webdav_config(f, app, area),
    }
}

/// 渲染设置主菜单
fn render_settings_main_menu(f: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new("设置")
        .style(Style::default().fg(Color::Magenta))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    let menu_options = ["删除小说", "清理孤立记录", "WebDAV同步配置"];
    let items: Vec<ListItem> = menu_options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let prefix = if Some(index) == app.settings.selected_option {
                ">> "
            } else {
                "   "
            };
            let display_text = format!("{}{}", prefix, option);
            ListItem::new(display_text).style(Style::default().fg(Color::White))
        })
        .collect();

    let menu_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("选择操作"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("");

    let list_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: area.height - 3,
    };

    let mut state = ListState::default();
    state.select(app.settings.selected_option);

    f.render_stateful_widget(menu_list, list_area, &mut state);

    let help_text = "↑/↓: 选择选项 | Enter: 确认 | Esc: 返回书架 | q: 退出";
    render_help_info(f, help_text, area);
}

/// 渲染删除小说菜单
fn render_delete_novel_menu(f: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new("删除小说")
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    if app.novels.is_empty() {
        let no_novels = Paragraph::new("没有发现小说文件")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("状态"));

        let content_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 3,
        };

        f.render_widget(no_novels, content_area);
    } else {
        let items: Vec<ListItem> = app
            .novels
            .iter()
            .enumerate()
            .map(|(index, novel)| {
                let prefix = if Some(index) == app.settings.selected_delete_novel_index {
                    ">> "
                } else {
                    "   "
                };
                let display_text = format!(
                    "{}{}",
                    prefix,
                    novel.path.file_stem().unwrap_or_default().to_string_lossy()
                );
                ListItem::new(display_text).style(Style::default().fg(Color::White))
            })
            .collect();

        let novel_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("小说列表 (共{}本)", app.novels.len())),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        let list_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 3,
        };

        let mut state = ListState::default();
        state.select(app.settings.selected_delete_novel_index);

        f.render_stateful_widget(novel_list, list_area, &mut state);
    }

    let help_text = if app.novels.is_empty() {
        "Esc: 返回设置菜单 | q: 退出"
    } else {
        "↑/↓: 选择小说 | D/d: 删除选中小说 | Esc: 返回设置菜单 | q: 退出"
    };
    render_help_info(f, help_text, area);
}

/// 渲染删除孤立记录菜单
fn render_delete_orphaned_menu(f: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new("清理孤立记录")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    if app.settings.orphaned_novels.is_empty() {
        let no_orphaned = Paragraph::new("没有发现孤立记录\n所有记录都对应有效的小说文件")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("状态"));

        let content_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 3,
        };

        f.render_widget(no_orphaned, content_area);
    } else {
        let items: Vec<ListItem> = app
            .settings
            .orphaned_novels
            .iter()
            .enumerate()
            .map(|(index, novel_info)| {
                let prefix = if Some(index) == app.settings.selected_orphaned_index {
                    ">> "
                } else {
                    "   "
                };
                let display_text = format!(
                    "{} {} ({})",
                    prefix,
                    novel_info.title,
                    novel_info.path.display()
                );
                ListItem::new(display_text).style(Style::default().fg(Color::Yellow))
            })
            .collect();

        let orphaned_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "孤立记录 (共{}条)",
                app.settings.orphaned_novels.len()
            )))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        let list_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 3,
        };

        let mut state = ListState::default();
        state.select(app.settings.selected_orphaned_index);

        f.render_stateful_widget(orphaned_list, list_area, &mut state);
    }

    let help_text = if app.settings.orphaned_novels.is_empty() {
        "Esc: 返回设置菜单 | q: 退出"
    } else {
        "↑/↓: 选择记录 | D/d: 删除选中记录 | Esc: 返回设置菜单 | q: 退出"
    };
    render_help_info(f, help_text, area);
}

/// 渲染WebDAV配置界面
fn render_webdav_config(f: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new("WebDAV 同步配置")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };
    f.render_widget(title, title_area);

    let config_state = &app.settings.webdav_config_state;
    let temp_config = &config_state.temp_config;
    let selected = config_state.selected_field;
    let edit_mode = config_state.edit_mode;

    let fields = [
        (
            "启用同步",
            if temp_config.enabled { "[✓]" } else { "[ ]" },
            0,
        ),
        ("URL", &temp_config.url, 1),
        ("用户名", &temp_config.username, 2),
        (
            "密码",
            if config_state.show_password {
                &temp_config.password
            } else {
                &"*".repeat(temp_config.password.len())
            },
            3,
        ),
        ("远程路径", &temp_config.remote_path, 4),
    ];

    let mut lines: Vec<Line> = vec![];
    for (label, value, idx) in fields {
        let is_selected = selected == idx;
        let is_editing = is_selected && edit_mode && idx > 0;

        let line_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let display_value = if is_editing {
            format!("{}_", value)
        } else {
            value.to_string()
        };

        let line_text = format!("{:10} {}", label, display_value);
        lines.push(Line::from(Span::styled(line_text, line_style)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(
            "按 'p' 切换密码显示 (当前: {})",
            if config_state.show_password {
                "显示"
            } else {
                "隐藏"
            }
        ),
        Style::default().fg(Color::Gray),
    )));

    match &config_state.connection_status {
        Some(Ok(())) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "✓ 连接成功",
                Style::default().fg(Color::Green),
            )));
        }
        Some(Err(msg)) => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("✗ 连接失败: {}", msg),
                Style::default().fg(Color::Red),
            )));
        }
        None => {}
    }

    let config_text = Text::from(lines);
    let config_paragraph = Paragraph::new(config_text)
        .block(Block::default().borders(Borders::ALL).title("配置"))
        .alignment(Alignment::Left);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: area.height - 4,
    };
    f.render_widget(config_paragraph, content_area);

    let help_text = if edit_mode {
        "输入文本 | Enter: 确认 | Esc: 取消编辑"
    } else {
        "↑/↓: 选择字段 | Enter: 编辑/切换启用 | S: 保存 | T: 测试连接 | P: 切换密码显示 | Esc: 返回 | q: 退出"
    };
    render_help_info(f, help_text, area);
}
