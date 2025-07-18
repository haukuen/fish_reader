use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::state::AppState;

use super::{bookshelf, chapter_list, reader, search, settings};

/// 渲染帮助信息的通用函数
///
/// # 参数
/// * `f` - 渲染框架
/// * `help_text` - 帮助文本内容
/// * `area` - 渲染区域
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

/// UI渲染函数
///
/// # 参数
/// * `f` - 渲染框架
/// * `app` - 应用状态
pub fn render(f: &mut Frame, app: &App) {
    match app.state {
        AppState::Bookshelf => bookshelf::render_bookshelf(f, app),
        AppState::Reading => reader::render_reader(f, app),
        AppState::Searching => search::render_search(f, app),
        AppState::ChapterList => chapter_list::render_chapter_list(f, app),
        AppState::Settings => settings::render_settings(f, app),
    }
}
