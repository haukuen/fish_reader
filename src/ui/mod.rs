pub mod bookshelf;
pub mod chapter_list;
pub mod reader;
pub mod search;
pub mod settings;

use ratatui::prelude::*;

use crate::app::App;
use crate::state::AppState;

pub fn render(f: &mut Frame, app: &App) {
    match app.state {
        AppState::Bookshelf => bookshelf::render_bookshelf(f, app),
        AppState::Reading => reader::render_reader(f, app),
        AppState::Searching => search::render_search(f, app),
        AppState::ChapterList => chapter_list::render_chapter_list(f, app),
        AppState::Settings => settings::render_settings(f, app),
    }
}
