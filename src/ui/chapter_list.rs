use ratatui::prelude::*;
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;

pub fn render_chapter_list(f: &mut Frame, app: &App) {
    let area = f.area();

    let title = Paragraph::new("章节目录")
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    if let Some(novel) = &app.current_novel {
        if novel.chapters.is_empty() {
            let no_chapters = Paragraph::new("未检测到章节信息\n\n可能原因：\n• 小说格式不规范\n• 章节标题格式特殊\n• 文件内容较短")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("提示"));

            let content_area = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width - 4,
                height: area.height - 3,
            };

            f.render_widget(no_chapters, content_area);
        } else {
            let items: Vec<ListItem> = novel
                .chapters
                .iter()
                .enumerate()
                .map(|(index, chapter)| {
                    let prefix = if Some(index) == app.selected_chapter_index {
                        ">> "
                    } else {
                        "   "
                    };
                    let display_text = format!("{}{}", prefix, chapter.title);
                    ListItem::new(display_text).style(Style::default().fg(Color::White))
                })
                .collect();

            let chapters_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("章节列表 (共{}章)", novel.chapters.len())),
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
            state.select(app.selected_chapter_index);

            if let Some(selected) = app.selected_chapter_index {
                let visible_height = list_area.height.saturating_sub(2) as usize;
                let half_height = visible_height / 2;

                if selected >= half_height {
                    let max_offset = novel.chapters.len().saturating_sub(visible_height);
                    let offset = (selected.saturating_sub(half_height)).min(max_offset);
                    state = state.with_offset(offset);
                }
            }

            f.render_stateful_widget(chapters_list, list_area, &mut state);
        }
    }

    let help_text = "↑/↓: 选择章节 | Enter: 跳转到章节 | Esc/q: 返回阅读";
    render_help_info(f, help_text, area);
}
