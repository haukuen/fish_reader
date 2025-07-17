use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn render_settings(f: &mut Frame, app: &App) {
    let area = f.area();

    // 创建设置页面标题
    let title = Paragraph::new("设置 - 清理孤立记录")
        .style(Style::default().fg(Color::Magenta))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    if app.orphaned_novels.is_empty() {
        // 没有孤立记录时显示提示信息
        let no_orphaned = Paragraph::new("没有发现孤立的小说记录")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("状态"));

        let content_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 5,
        };

        f.render_widget(no_orphaned, content_area);
    } else {
        // 显示孤立记录列表
        let items: Vec<ListItem> = app
            .orphaned_novels
            .iter()
            .enumerate()
            .map(|(index, novel_info)| {
                let prefix = if Some(index) == app.selected_orphaned_index {
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("孤立记录 (共{}条)", app.orphaned_novels.len())),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        let list_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 5,
        };

        let mut state = ListState::default();
        state.select(app.selected_orphaned_index);

        f.render_stateful_widget(orphaned_list, list_area, &mut state);
    }

    // 创建帮助信息
    let help_text = if app.orphaned_novels.is_empty() {
        "Esc/q: 返回书架"
    } else {
        "↑/↓: 选择记录 | D/d: 删除选中记录 | Esc/q: 返回书架"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    let help_area = Rect {
        x: area.x,
        y: area.height - 3,
        width: area.width,
        height: 3,
    };

    f.render_widget(help, help_area);
}
