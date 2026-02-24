use ratatui::prelude::*;
use ratatui::style::Modifier;
use ratatui::widgets::*;

use crate::app::App;
use crate::state::AppState;

use super::{bookmark, bookshelf, chapter_list, reader, search, settings, sync_status};

pub fn render_help_info(f: &mut Frame, help_text: &str, area: Rect) {
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);

    let help_area = Rect {
        x: area.x,
        y: area.height - 1,
        width: area.width,
        height: 1,
    };

    f.render_widget(help, help_area);
}

pub fn render_error_message(f: &mut Frame, error_msg: &str, area: Rect) {
    let error = Paragraph::new(format!("⚠ {}", error_msg))
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    let error_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };

    f.render_widget(error, error_area);
}

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let status_area = main_layout[1];

    match app.state {
        AppState::Bookshelf => bookshelf::render_bookshelf(f, app),
        AppState::Reading => reader::render_reader(f, app),
        AppState::Searching => search::render_search(f, app),
        AppState::ChapterList => chapter_list::render_chapter_list(f, app),
        AppState::Settings => settings::render_settings(f, app),
        AppState::BookmarkList | AppState::BookmarkAdd => bookmark::render_bookmark(f, app),
    }

    let sync_widget = sync_status::SyncStatusWidget {
        status: app.sync_status.clone(),
    };
    sync_widget.render(status_area, f.buffer_mut());

    if let Some(ref error_msg) = app.error_message {
        render_error_message(f, error_msg, area);
    }
}
