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
            SyncStatus::InProgress(msg) | SyncStatus::Success(msg) | SyncStatus::Error(msg) => msg,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_text_mapping() {
        assert_eq!(SyncStatus::Idle.text(), "");
        assert_eq!(
            SyncStatus::InProgress("loading".to_string()).text(),
            "loading"
        );
        assert_eq!(SyncStatus::Success("ok".to_string()).text(), "ok");
        assert_eq!(SyncStatus::Error("bad".to_string()).text(), "bad");
    }

    #[test]
    fn test_sync_status_color_mapping() {
        assert_eq!(SyncStatus::Idle.color(), Color::Gray);
        assert_eq!(
            SyncStatus::InProgress("x".to_string()).color(),
            Color::Yellow
        );
        assert_eq!(SyncStatus::Success("x".to_string()).color(), Color::Green);
        assert_eq!(SyncStatus::Error("x".to_string()).color(), Color::Red);
    }

    #[test]
    fn test_sync_status_is_busy_only_for_in_progress() {
        assert!(!SyncStatus::Idle.is_busy());
        assert!(SyncStatus::InProgress("x".to_string()).is_busy());
        assert!(!SyncStatus::Success("x".to_string()).is_busy());
        assert!(!SyncStatus::Error("x".to_string()).is_busy());
    }
}
