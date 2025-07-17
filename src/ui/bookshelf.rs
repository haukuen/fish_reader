use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn render_bookshelf(f: &mut Frame, app: &App) {
    let area = f.area();

    // 创建书架标题
    let title = Paragraph::new("书架")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    // 创建小说列表
    let items: Vec<ListItem> = app
        .novels
        .iter()
        .enumerate()
        .map(|(index, novel)| {
            let prefix = if Some(index) == app.selected_novel_index {
                ">> "
            } else {
                "   "
            };
            ListItem::new(format!("{}{}", prefix, novel.title))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let novels_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("可用小说"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("");

    let list_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: area.height - 3,
    };

    let mut state = ListState::default();
    state.select(app.selected_novel_index);

    f.render_stateful_widget(novels_list, list_area, &mut state);

    // 创建帮助信息
    let help_text = "↑/k: 上移  ↓/j: 下移  Enter: 选择  s: 设置  Esc/q: 退出";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    let help_area = Rect {
        x: area.x,
        y: area.height - 1,
        width: area.width,
        height: 1,
    };

    f.render_widget(help, help_area);
}
