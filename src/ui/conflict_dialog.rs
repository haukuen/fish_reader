use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct ConflictDialog {
    pub local_version: u64,
    pub remote_version: u64,
    pub selected_option: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolution {
    UseLocal,
    UseRemote,
    Merge,
}

impl ConflictDialog {
    pub fn new(local_version: u64, remote_version: u64) -> Self {
        Self {
            local_version,
            remote_version,
            selected_option: 1, // Default to Remote
        }
    }

    pub fn next_option(&mut self) {
        self.selected_option = (self.selected_option + 1) % 3;
    }

    pub fn prev_option(&mut self) {
        self.selected_option = (self.selected_option + 2) % 3;
    }

    pub fn get_resolution(&self) -> ConflictResolution {
        match self.selected_option {
            0 => ConflictResolution::UseLocal,
            1 => ConflictResolution::UseRemote,
            _ => ConflictResolution::Merge,
        }
    }
}

impl Widget for &ConflictDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("版本冲突")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = vec![
            Line::from(""),
            Line::from(format!("本地版本: {}", self.local_version)),
            Line::from(format!("远程版本: {}", self.remote_version)),
            Line::from(""),
            Line::from("远程有更新的数据，请选择处理方式:"),
            Line::from(""),
            self.render_option(0, "[L] 使用本地版本 (覆盖远程)"),
            self.render_option(1, "[R] 使用远程版本 (下载并覆盖本地)"),
            self.render_option(2, "[M] 保留两者 (手动处理)"),
            Line::from(""),
            Line::from("使用 ↑↓ 选择，Enter 确认"),
        ];

        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(inner, buf);
    }
}

impl ConflictDialog {
    fn render_option(&self, index: usize, text: &str) -> Line {
        let style = if self.selected_option == index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Line::from(Span::styled(format!("  {}  ", text), style))
    }
}
