use ratatui::prelude::*;
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;

pub fn render_reader(f: &mut Frame, app: &App) {
    if let Some(novel) = &app.current_novel {
        let area = f.area();

        // 计算可见内容区域
        let content_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width - 2,
            height: area.height - 1,
        };

        // 分割内容为行
        let lines: Vec<&str> = novel.content.lines().collect();

        // 计算可见行数
        let visible_height = content_area.height as usize;
        let start_line = novel.progress.scroll_offset;
        let end_line = (start_line + visible_height).min(lines.len());

        // 创建段落显示内容
        let visible_content = lines[start_line..end_line].join("\n");
        let content = Paragraph::new(visible_content)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });

        f.render_widget(content, content_area);

        // 创建帮助信息（贴近底部）
        let progress_text = format!("{}/{}", start_line + 1, lines.len());
        let bookmark_count = novel.progress.bookmarks.len();
        let bookmark_info = if bookmark_count > 0 {
            format!(" 签:{}", bookmark_count)
        } else {
            String::new()
        };

        // 根据终端宽度自适应帮助信息
        let width = area.width as usize;
        let help_text = if width >= 100 {
            // 宽屏：完整信息
            format!(
                "{}行{} │ jk:滚动 hl:翻页 /:搜索 t:目录 b:书签 m:标记 Esc:返回 q:退出",
                progress_text, bookmark_info
            )
        } else if width >= 70 {
            // 中等：省略部分
            format!(
                "{}行{} │ jk:滚动 hl:翻页 /:搜 t:目录 b:签 m:标 q:退",
                progress_text, bookmark_info
            )
        } else if width >= 50 {
            // 窄屏：最常用
            format!("{}行 │ jk:滚 hl:翻 /:搜 t:目录 q:退", progress_text)
        } else {
            // 极窄：仅进度
            format!("{}行", progress_text)
        };
        render_help_info(f, &help_text, area);
    }
}
