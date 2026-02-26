use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;

/// 创建带高亮的文本行
/// # 参数
/// - `text`: 原始文本
/// - `search_term`: 搜索关键词
/// # 返回
/// 返回包含高亮显示的Line对象
fn create_highlighted_line(text: &str, search_term: &str) -> Line<'static> {
    if search_term.is_empty() {
        return Line::from(text.to_string());
    }

    let mut spans = Vec::new();
    let text_lower = text.to_lowercase();
    let search_lower = search_term.to_lowercase();
    let mut last_end = 0;

    while let Some(start) = text_lower[last_end..].find(&search_lower) {
        let actual_start = last_end + start;
        let actual_end = actual_start + search_term.len();

        if actual_start > last_end {
            spans.push(Span::styled(
                text[last_end..actual_start].to_string(),
                Style::default().fg(Color::White),
            ));
        }

        spans.push(Span::styled(
            text[actual_start..actual_end].to_string(),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));

        last_end = actual_end;
    }

    if last_end < text.len() {
        spans.push(Span::styled(
            text[last_end..].to_string(),
            Style::default().fg(Color::White),
        ));
    }

    Line::from(spans)
}

pub fn render_search(f: &mut Frame, app: &App) {
    let area = f.area();

    let title = Paragraph::new("搜索模式")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    let search_text = format!("搜索: {}", app.search.input);
    let search_input = Paragraph::new(search_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("输入搜索内容"));

    let input_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: 3,
    };

    f.render_widget(search_input, input_area);

    if !app.search.results.is_empty() {
        let items: Vec<ListItem> = app
            .search
            .results
            .iter()
            .enumerate()
            .map(|(index, (line_num, content))| {
                let prefix = if Some(index) == app.search.selected_index {
                    ">> "
                } else {
                    "   "
                };

                let line_prefix = format!("{}{}: ", prefix, line_num + 1);
                let mut line_spans =
                    vec![Span::styled(line_prefix, Style::default().fg(Color::Cyan))];

                let highlighted_line = create_highlighted_line(content.trim(), &app.search.input);
                line_spans.extend(highlighted_line.spans);

                ListItem::new(Line::from(line_spans))
            })
            .collect();

        let results_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("搜索结果"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        let results_area = Rect {
            x: area.x + 2,
            y: area.y + 5,
            width: area.width - 4,
            height: area.height - 6,
        };

        let mut state = ListState::default();
        state.select(app.search.selected_index);

        if let Some(selected) = app.search.selected_index {
            let visible_height = results_area.height.saturating_sub(2) as usize;
            let half_height = visible_height / 2;

            if selected >= half_height {
                let max_offset = app.search.results.len().saturating_sub(visible_height);
                let offset = (selected.saturating_sub(half_height)).min(max_offset);
                state = state.with_offset(offset);
            }
        }

        f.render_stateful_widget(results_list, results_area, &mut state);
    }

    let help_text = "输入搜索内容 | ↑/↓: 选择结果 | Enter: 跳转 | Esc: 返回阅读";
    render_help_info(f, help_text, area);
}
