use ratatui::prelude::*;
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;

pub fn render_reader(f: &mut Frame, app: &App) {
    if let Some(novel) = &app.current_novel {
        let area = f.area();

        let content_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width - 2,
            height: area.height - 1,
        };

        let total_lines = novel.line_count();
        let spacing = novel.progress.line_spacing;
        let line_physical_height = spacing + 1;

        let visible_height = content_area.height as usize;
        let start_line = novel
            .progress
            .scroll_offset
            .min(total_lines.saturating_sub(1));
        let visible_line_count = visible_height / line_physical_height;
        let end_line = (start_line + visible_line_count).min(total_lines);

        let visible_content = if start_line < total_lines {
            let sep = "\n".repeat(spacing + 1);
            novel.lines()[start_line..end_line].join(&sep)
        } else {
            String::new()
        };
        let content = Paragraph::new(visible_content)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });

        f.render_widget(content, content_area);

        let percent = ((start_line + 1) * 100)
            .checked_div(total_lines)
            .unwrap_or(0);
        let progress_text = format!("{}/{}({}%)", start_line + 1, total_lines, percent);
        let bookmark_count = novel.progress.bookmarks.len();
        let bookmark_info = if bookmark_count > 0 {
            format!(" 签:{}", bookmark_count)
        } else {
            String::new()
        };

        let spacing_info = if spacing > 0 {
            format!(" 间距:{}", spacing)
        } else {
            String::new()
        };

        let width = area.width as usize;
        let help_text = if width >= 100 {
            format!(
                "{}行{}{} │ jk:滚动 hl:翻页 []:章节 /:搜索 t:目录 b:书签 m:标记 Esc:返回 q:退出",
                progress_text, bookmark_info, spacing_info
            )
        } else if width >= 70 {
            format!(
                "{}行{}{} │ jk:滚动 hl:翻页 []:章节 /:搜 t:目录 b:签 m:标 q:退",
                progress_text, bookmark_info, spacing_info
            )
        } else if width >= 50 {
            format!(
                "{}行{} │ jk:滚 hl:翻 []:章 /:搜 t:目录 q:退",
                progress_text, spacing_info
            )
        } else {
            format!("{}行{}", progress_text, spacing_info)
        };
        render_help_info(f, &help_text, area);
    }
}
