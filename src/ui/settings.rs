use ratatui::prelude::*;
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
    }
}

/// 渲染设置主菜单
fn render_settings_main_menu(f: &mut Frame, app: &App, area: Rect) {
    // 创建设置页面标题
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

    // 创建菜单选项
    let menu_options = ["删除小说", "清理孤立记录"];
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

    // 创建帮助信息
    let help_text = "↑/↓: 选择选项 | Enter: 确认 | Esc/q: 返回书架";
    render_help_info(f, help_text, area);
}

/// 渲染删除小说菜单
fn render_delete_novel_menu(f: &mut Frame, app: &App, area: Rect) {
    // 创建标题
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
        // 没有小说时显示提示信息
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
        // 显示小说列表
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
                let display_text = format!("{}{}", prefix, novel.title);
                ListItem::new(display_text).style(Style::default().fg(Color::White))
            })
            .collect();

        let novels_list = List::new(items)
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

        f.render_stateful_widget(novels_list, list_area, &mut state);
    }

    // 创建帮助信息
    let help_text = if app.novels.is_empty() {
        "Esc/q: 返回设置菜单"
    } else {
        "↑/↓: 选择小说 | D/d: 删除选中小说 | Esc/q: 返回设置菜单"
    };
    render_help_info(f, help_text, area);
}

/// 渲染删除孤立记录菜单
fn render_delete_orphaned_menu(f: &mut Frame, app: &App, area: Rect) {
    // 创建标题
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
        // 没有孤立记录时显示提示信息
        let no_orphaned = Paragraph::new("没有发现孤立的小说记录")
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
        // 显示孤立记录列表
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

    // 创建帮助信息
    let help_text = if app.settings.orphaned_novels.is_empty() {
        "Esc/q: 返回设置菜单"
    } else {
        "↑/↓: 选择记录 | D/d: 删除选中记录 | Esc/q: 返回设置菜单"
    };
    render_help_info(f, help_text, area);
}
