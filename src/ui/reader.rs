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
        let progress_text = format!("进度: {}/{} 行", start_line + 1, lines.len());
        let bookmark_count = novel.progress.bookmarks.len();
        let bookmark_info = if bookmark_count > 0 {
            format!(" | 书签: {}个", bookmark_count)
        } else {
            String::new()
        };
        let help_text = format!(
            "{}{} | ↑/k: 上滚  ↓/j: 下滚  ←/h: 上页  →/l: 下页  /: 搜索  t: 章节  b: 书签  m: 添加书签  Esc: 返回  q: 退出",
            progress_text, bookmark_info
        );
        render_help_info(f, &help_text, area);
    }
}
