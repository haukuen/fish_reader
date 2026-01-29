mod app;
mod config;
mod event;
mod model;
mod state;
mod ui;

use anyhow::{Context, Result};
use clap::Command;
use crossterm::ExecutableCommand;
use crossterm::event::{self as crossterm_event, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use std::io::{stdout, Stdout};
use std::time::{Duration, Instant};

use crate::app::App;

/// 终端守卫，确保程序退出时（包括 panic）正确恢复终端状态
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(DisableMouseCapture);
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

fn main() -> Result<()> {
    Command::new("fish_reader")
        .version(env!("CARGO_PKG_VERSION"))
        .author("haukuen")
        .about("A terminal-based novel reader with bookshelf management")
        .get_matches();

    let mut app = App::new().context("创建应用失败")?;
    run(&mut app).context("运行应用失败")?;

    Ok(())
}

fn run(app: &mut App) -> Result<()> {
    // 使用 RAII 模式管理终端状态，确保 panic 时也能正确恢复
    let mut guard = TerminalGuard::new()?;

    // 主循环
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        let size = guard.terminal.size()?;
        app.terminal_size = Rect::new(0, 0, size.width, size.height);

        guard.terminal.draw(|f| ui::render(f, app))?;

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
            .update_novel_progress(&novel.path, novel.progress.clone());
    }
    if let Err(e) = app.library.save() {
        eprintln!("Failed to save progress: {}", e);
    }

    // guard 在此处 drop，自动恢复终端状态
    Ok(())
}
