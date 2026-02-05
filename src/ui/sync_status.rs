use ratatui::{prelude::*, style::Color, widgets::Paragraph};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error,
    Success,
}

impl SyncStatus {
    pub fn text(&self) -> &str {
        match self {
            SyncStatus::Idle => "同步: 就绪",
            SyncStatus::Syncing => "同步: 进行中...",
            SyncStatus::Error => "同步: 错误",
            SyncStatus::Success => "同步: 完成",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SyncStatus::Idle => Color::Gray,
            SyncStatus::Syncing => Color::Yellow,
            SyncStatus::Error => Color::Red,
            SyncStatus::Success => Color::Green,
        }
    }
}

pub struct SyncStatusWidget {
    pub status: SyncStatus,
}

impl Widget for SyncStatusWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = self.status.text();
        let style = Style::default().fg(self.status.color());

        Paragraph::new(text).style(style).render(area, buf);
    }
}
