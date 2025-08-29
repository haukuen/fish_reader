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

    // 查找所有匹配位置
    while let Some(start) = text_lower[last_end..].find(&search_lower) {
        let actual_start = last_end + start;
        let actual_end = actual_start + search_term.len();

        // 添加匹配前的普通文本
        if actual_start > last_end {
            spans.push(Span::styled(
                text[last_end..actual_start].to_string(),
                Style::default().fg(Color::White),
            ));
        }

        // 添加高亮的匹配文本
        spans.push(Span::styled(
            text[actual_start..actual_end].to_string(),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));

        last_end = actual_end;
    }

    // 添加剩余的普通文本
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

    // 创建搜索标题
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

    // 创建搜索输入框
    let search_text = format!("搜索: {}", app.search_input);
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

    // 创建搜索结果列表
    if !app.search_results.is_empty() {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(index, (line_num, content))| {
                let prefix = if Some(index) == app.selected_search_index {
                    ">> "
                } else {
                    "   "
                };

                // 创建行号前缀
                let line_prefix = format!("{}{}: ", prefix, line_num + 1);
                let mut line_spans =
                    vec![Span::styled(line_prefix, Style::default().fg(Color::Cyan))];

                // 创建高亮的内容部分
                let highlighted_line = create_highlighted_line(content.trim(), &app.search_input);
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
        state.select(app.selected_search_index);

        // 计算滚动偏移，让选中的搜索结果显示在中间位置
        if let Some(selected) = app.selected_search_index {
            let visible_height = results_area.height.saturating_sub(2) as usize; // 减去边框
            let half_height = visible_height / 2;

            if selected >= half_height {
                let max_offset = app.search_results.len().saturating_sub(visible_height);
                let offset = (selected.saturating_sub(half_height)).min(max_offset);
                state = state.with_offset(offset);
            }
        }

        f.render_stateful_widget(results_list, results_area, &mut state);
    }

    // 创建帮助信息
    let help_text = "输入搜索内容 | ↑/↓: 选择结果 | Enter: 跳转 | Esc/q: 返回";
    render_help_info(f, help_text, area);
}
