use ratatui::prelude::*;
use ratatui::widgets::*;

use super::utils::render_help_info;
use crate::app::App;
use crate::state::AppState;

/// 渲染书签管理界面
/// # 参数
/// - `f`: 渲染框架
/// - `app`: 应用状态
pub fn render_bookmark(f: &mut Frame, app: &App) {
    match app.state {
        AppState::BookmarkList => render_bookmark_list(f, app),
        AppState::BookmarkAdd => render_bookmark_add(f, app),
        _ => {}
    }
}

/// 渲染书签列表界面
/// # 参数
/// - `f`: 渲染框架
/// - `app`: 应用状态
fn render_bookmark_list(f: &mut Frame, app: &App) {
    let area = f.area();

    // 创建书签列表标题
    let title = Paragraph::new("书签管理")
        .style(Style::default().fg(Color::Blue))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    // 显示书签列表
    if let Some(bookmarks) = app.get_current_bookmarks() {
        if bookmarks.is_empty() {
            // 没有书签时显示提示信息
            let no_bookmarks = Paragraph::new(
                "暂无书签\n\n按 'a' 或 'A' 添加书签\n按 'm' 或 'M' 在阅读时快速添加书签",
            )
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("提示"));

            let content_area = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width - 4,
                height: area.height - 3,
            };

            f.render_widget(no_bookmarks, content_area);
        } else {
            // 创建书签列表
            let items: Vec<ListItem> = bookmarks
                .iter()
                .enumerate()
                .map(|(index, bookmark)| {
                    let prefix = if Some(index) == app.selected_bookmark_index {
                        ">> "
                    } else {
                        "   "
                    };

                    // 格式化书签信息
                    let display_text = format!(
                        "{}{} (行: {})",
                        prefix,
                        bookmark.name,
                        bookmark.position + 1
                    );

                    ListItem::new(display_text).style(Style::default().fg(Color::White))
                })
                .collect();

            let bookmarks_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("书签列表 (共{}个)", bookmarks.len())),
                )
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("");

            let list_area = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width - 4,
                height: area.height - 3,
            };

            let mut state = ListState::default();
            state.select(app.selected_bookmark_index);

            // 计算滚动偏移，让选中的书签显示在中间位置
            if let Some(selected) = app.selected_bookmark_index {
                let visible_height = list_area.height.saturating_sub(2) as usize; // 减去边框
                let half_height = visible_height / 2;

                if selected >= half_height {
                    let max_offset = bookmarks.len().saturating_sub(visible_height);
                    let offset = (selected.saturating_sub(half_height)).min(max_offset);
                    state = state.with_offset(offset);
                }
            }

            f.render_stateful_widget(bookmarks_list, list_area, &mut state);
        }
    }

    // 创建帮助信息
    let help_text = if app.get_current_bookmarks().map_or(true, |b| b.is_empty()) {
        "a: 添加书签 | Esc/q: 返回阅读"
    } else {
        "↑/↓: 选择书签 | Enter: 跳转 | d: 删除 | a: 添加 | Esc/q: 返回阅读"
    };
    render_help_info(f, help_text, area);
}

/// 渲染添加书签界面
/// # 参数
/// - `f`: 渲染框架
/// - `app`: 应用状态
fn render_bookmark_add(f: &mut Frame, app: &App) {
    let area = f.area();

    // 创建添加书签标题
    let title = Paragraph::new("添加书签")
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);

    let title_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 2,
    };

    f.render_widget(title, title_area);

    // 显示当前位置信息
    let position_info = if let Some(novel) = &app.current_novel {
        format!("当前位置: 第 {} 行", novel.progress.scroll_offset + 1)
    } else {
        "当前位置: 未知".to_string()
    };

    let position_paragraph = Paragraph::new(position_info)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("位置信息"));

    let position_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width - 4,
        height: 3,
    };

    f.render_widget(position_paragraph, position_area);

    // 创建书签名称输入框
    let name_text = format!("书签名称: {}", app.bookmark_input);
    let name_input = Paragraph::new(name_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("输入书签名称"));

    let name_area = Rect {
        x: area.x + 2,
        y: area.y + 5,
        width: area.width - 4,
        height: 3,
    };

    f.render_widget(name_input, name_area);

    // 创建帮助信息
    let help_text = "输入书签名称 | Enter: 确认添加 | Esc/q: 取消";
    render_help_info(f, help_text, area);
}
