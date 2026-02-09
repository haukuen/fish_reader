use ratatui::{prelude::*, style::Color, widgets::Paragraph};

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Idle,
    InProgress(String),
    Success(String),
    Error(String),
}

impl SyncStatus {
    pub fn text(&self) -> &str {
        match self {
            SyncStatus::Idle => "",
            SyncStatus::InProgress(msg) | SyncStatus::Success(msg) | SyncStatus::Error(msg) => {
                msg
            }
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SyncStatus::Idle => Color::Gray,
            SyncStatus::InProgress(_) => Color::Yellow,
            SyncStatus::Success(_) => Color::Green,
            SyncStatus::Error(_) => Color::Red,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, SyncStatus::InProgress(_))
    }
}

pub struct SyncStatusWidget {
    pub status: SyncStatus,
}

impl Widget for SyncStatusWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = self.status.text();
        if text.is_empty() {
            return;
        }
        let style = Style::default().fg(self.status.color());
        Paragraph::new(text).style(style).render(area, buf);
    }
}
