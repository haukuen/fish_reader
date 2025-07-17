mod app;
mod event;
mod model;
mod state;
mod ui;

use anyhow::{Context, Result};
use clap::Command;
use crossterm::ExecutableCommand;
use crossterm::event::{self as crossterm_event, Event, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::{Duration, Instant};

use crate::app::App;

fn main() -> Result<()> {
    let _matches = Command::new("fish_reader")
        .version(env!("CARGO_PKG_VERSION"))
        .author("haukuen")
        .about("A terminal-based novel reader with bookshelf management")
        .get_matches();

    let mut app = App::new().context("创建应用失败")?;
    run(&mut app).context("运行应用失败")?;

    Ok(())
}

fn run(app: &mut App) -> Result<()> {
    // 设置终端
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // 主循环
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        let size = terminal.size()?;
        app.terminal_size = Rect::new(0, 0, size.width, size.height);

        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm_event::poll(timeout)? {
            match crossterm_event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        event::handle_key(app, key.code);
                    }
                }
                Event::Mouse(mouse) => {
                    event::handle_mouse(app, mouse);
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // 保存阅读进度
    if let Some(novel) = &app.current_novel {
        app.library
            .update_novel_progress(&novel.path, novel.progress);
    }
    let _ = app.library.save();

    // 恢复终端
    disable_raw_mode()?;
    stdout()
        .execute(crossterm::event::DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;

    Ok(())
}
