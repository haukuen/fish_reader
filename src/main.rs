mod model;

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use model::{Library, Novel};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone)]
enum AppState {
    Bookshelf,
    Reading,
    Searching,
}

struct App {
    /// 当前应用状态（书架/阅读/搜索模式）
    state: AppState,
    /// 持久化存储处理器
    library: Library,
    /// 发现的小说列表
    novels: Vec<Novel>,
    /// 书架选中的小说索引
    selected_novel_index: Option<usize>,
    /// 当前正在阅读的小说
    current_novel: Option<Novel>,
    /// 退出标志位
    should_quit: bool,
    /// 终端尺寸缓存
    terminal_size: Rect,
    /// 搜索输入框内容
    search_input: String,
    /// 搜索结果列表（行号，内容）
    search_results: Vec<(usize, String)>,
    /// 当前选中的搜索结果索引
    selected_search_index: Option<usize>,
    /// 上一个状态（用于从搜索返回）
    previous_state: AppState,
}

impl App {
    /// 初始化应用程序
    /// # 流程
    /// 1. 加载历史进度 2. 扫描小说目录
    fn new() -> Result<Self> {
        // 加载阅读进度
        let library = Library::load();

        // 获取小说文件
        let novels_dir = Self::get_novels_dir();
        let novels = Self::load_novels_from_dir(&novels_dir)?;

        Ok(App {
            state: AppState::Bookshelf,
            library,
            novels,
            selected_novel_index: None,
            current_novel: None,
            should_quit: false,
            terminal_size: Rect::default(),
            search_input: String::new(),
            search_results: Vec::new(),
            selected_search_index: None,
            previous_state: AppState::Bookshelf,
        })
    }

    fn get_novels_dir() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".fish_reader");
        path.push("novels");

        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
        }

        path
    }

    fn load_novels_from_dir(dir: &Path) -> Result<Vec<Novel>> {
        let mut novels = Vec::new();

        if !dir.exists() {
            return Ok(novels);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                let mut novel = Novel::new(path);
                let _ = novel.load_content(); // 忽略加载错误，继续处理其他小说
                novels.push(novel);
            }
        }

        Ok(novels)
    }

    fn run(&mut self) -> Result<()> {
        // 设置终端
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        // 主循环
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();

        while !self.should_quit {
            let size = terminal.size()?;
            self.terminal_size = Rect::new(0, 0, size.width, size.height);

            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        // 保存阅读进度
        if let Some(novel) = &self.current_novel {
            self.library
                .update_novel_progress(&novel.path, novel.progress);
        }
        let _ = self.library.save();

        // 恢复终端
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;

        Ok(())
    }

    fn ui(&self, f: &mut Frame) {
        match self.state {
            AppState::Bookshelf => self.render_bookshelf(f),
            AppState::Reading => self.render_reader(f),
            AppState::Searching => self.render_search(f),
        }
    }

    fn render_bookshelf(&self, f: &mut Frame) {
        let area = f.area();

        // 创建书架标题
        let title = Paragraph::new("小说阅读器 - 书架")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 2,
        };

        f.render_widget(title, title_area);

        // 创建小说列表
        let items: Vec<ListItem> = self
            .novels
            .iter()
            .enumerate()
            .map(|(index, novel)| {
                let prefix = if Some(index) == self.selected_novel_index {
                    ">> "
                } else {
                    "   "
                };
                ListItem::new(format!("{}{}", prefix, novel.title))
                    .style(Style::default().fg(Color::White))
            })
            .collect();

        let novels_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("可用小说"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("");

        let list_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: area.height - 5,
        };

        let mut state = ListState::default();
        state.select(self.selected_novel_index);

        f.render_stateful_widget(novels_list, list_area, &mut state);

        // 创建帮助信息
        let help_text = "↑/k: 上移  ↓/j: 下移  Enter: 选择  q: 退出";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        let help_area = Rect {
            x: area.x,
            y: area.height - 3,
            width: area.width,
            height: 3,
        };

        f.render_widget(help, help_area);
    }

    fn render_reader(&self, f: &mut Frame) {
        if let Some(novel) = &self.current_novel {
            let area = f.area();

            // 创建标题
            let title = Paragraph::new(format!("阅读中: {}", novel.title))
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center);

            let title_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 2,
            };

            f.render_widget(title, title_area);

            // 计算可见内容区域
            let content_area = Rect {
                x: area.x + 1,
                y: area.y + 2,
                width: area.width - 2,
                height: area.height - 5,
            };

            // 分割内容为行
            let lines: Vec<&str> = novel.content.lines().collect();

            // 计算可见行数
            let visible_height = content_area.height as usize;
            let start_line = novel.progress.scroll_offset;
            let end_line = (start_line + visible_height).min(lines.len());

            // 创建段落显示内容
            let visible_content = lines[start_line..end_line].join("\n");
            let content = Paragraph::new(visible_content)
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL))
                .wrap(Wrap { trim: false });

            f.render_widget(content, content_area);

            // 创建帮助信息
            let progress_text = format!("进度: {}/{} 行", start_line + 1, lines.len());
            let help_text = format!(
                "{} | ↑/k: 上滚  ↓/j: 下滚  ←/h: 上页  →/l: 下页  /: 搜索  p: 返回书架  q: 退出",
                progress_text
            );
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);

            let help_area = Rect {
                x: area.x,
                y: area.height - 3,
                width: area.width,
                height: 3,
            };

            f.render_widget(help, help_area);
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match self.state {
            AppState::Bookshelf => self.handle_bookshelf_key(key),
            AppState::Reading => self.handle_reader_key(key),
            AppState::Searching => self.handle_search_key(key),
        }
    }

    fn handle_bookshelf_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.novels.is_empty() {
                    let current = self.selected_novel_index.unwrap_or(0);
                    let next = if current > 0 {
                        current - 1
                    } else {
                        self.novels.len() - 1
                    };
                    self.selected_novel_index = Some(next);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.novels.is_empty() {
                    // 如果当前没有选中任何小说，则选中第一本
                    if self.selected_novel_index.is_none() {
                        self.selected_novel_index = Some(0);
                    } else {
                        let current = self.selected_novel_index.unwrap();
                        let next = (current + 1) % self.novels.len();
                        self.selected_novel_index = Some(next);
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(index) = self.selected_novel_index {
                    if index < self.novels.len() {
                        let mut novel = self.novels[index].clone();

                        // 恢复阅读进度
                        novel.progress = self.library.get_novel_progress(&novel.path);

                        self.current_novel = Some(novel);
                        self.state = AppState::Reading;
                    }
                }
            }
            _ => {}
        }
    }

    /// 处理阅读器模式下的键盘事件
    /// # 参数
    /// - `key`: 按下的键位代码
    /// # 功能
    /// 实现滚动控制、进度保存和界面切换
    fn handle_reader_key(&mut self, key: KeyCode) {
        if let Some(novel) = &mut self.current_novel {
            let lines: Vec<&str> = novel.content.lines().collect();
            let max_scroll = lines.len().saturating_sub(1);

            // 精确计算内容区域高度（标题2行 + 帮助信息3行 + 边框）
            let content_height = (self.terminal_size.height as usize)
                .saturating_sub(2 + 3) // 标题2行 + 帮助信息3行
                .saturating_sub(2) // 上下边框各占1行
                .saturating_sub(1); // 减去1行，方便阅读
            let page_size = content_height.max(1);

            match key {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // 保存阅读进度
                    self.library
                        .update_novel_progress(&novel.path, novel.progress);
                    let _ = self.library.save();
                    self.should_quit = true;
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    // 保存阅读进度
                    self.library
                        .update_novel_progress(&novel.path, novel.progress);
                    let _ = self.library.save();
                    self.state = AppState::Bookshelf;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    // 向上滚动一行
                    if novel.progress.scroll_offset > 0 {
                        novel.progress.scroll_offset -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // 向下滚动一行
                    if novel.progress.scroll_offset < max_scroll.saturating_sub(page_size) {
                        novel.progress.scroll_offset += 1;
                    }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    // 向上翻页（使用修正后的页面尺寸）
                    novel.progress.scroll_offset =
                        novel.progress.scroll_offset.saturating_sub(page_size);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    // 精确边界控制：确保不超过最大可滚动行数
                    let max_offset = lines.len().saturating_sub(content_height);
                    novel.progress.scroll_offset =
                        (novel.progress.scroll_offset + page_size).min(max_offset);
                }
                KeyCode::Char('/') => {
                    // 进入搜索模式
                    self.previous_state = AppState::Reading;
                    self.state = AppState::Searching;
                    self.search_input.clear();
                    self.search_results.clear();
                    self.selected_search_index = None;
                }
                _ => {}
            }
        }
    }

    /// 渲染搜索界面
    /// # 功能
    /// 显示搜索输入框和搜索结果列表
    fn render_search(&self, f: &mut Frame) {
        let area = f.area();

        // 创建搜索标题
        let title = Paragraph::new("搜索模式")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 2,
        };

        f.render_widget(title, title_area);

        // 创建搜索输入框
        let search_text = format!("搜索: {}", self.search_input);
        let search_input = Paragraph::new(search_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("输入搜索内容"));

        let input_area = Rect {
            x: area.x + 2,
            y: area.y + 2,
            width: area.width - 4,
            height: 3,
        };

        f.render_widget(search_input, input_area);

        // 创建搜索结果列表
        if !self.search_results.is_empty() {
            let items: Vec<ListItem> = self
                .search_results
                .iter()
                .enumerate()
                .map(|(index, (line_num, content))| {
                    let prefix = if Some(index) == self.selected_search_index {
                        ">> "
                    } else {
                        "   "
                    };
                    let display_text = format!("{}{}: {}", prefix, line_num + 1, content.trim());
                    ListItem::new(display_text).style(Style::default().fg(Color::White))
                })
                .collect();

            let results_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("搜索结果"))
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("");

            let results_area = Rect {
                x: area.x + 2,
                y: area.y + 5,
                width: area.width - 4,
                height: area.height - 8,
            };

            let mut state = ListState::default();
            state.select(self.selected_search_index);

            f.render_stateful_widget(results_list, results_area, &mut state);
        }

        // 创建帮助信息
         let help_text = "输入搜索内容 | ↑/↓: 选择结果 | Enter: 跳转 | Esc: 返回";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        let help_area = Rect {
            x: area.x,
            y: area.height - 3,
            width: area.width,
            height: 3,
        };

        f.render_widget(help, help_area);
    }

    /// 处理搜索模式下的键盘事件
    /// # 参数
    /// - `key`: 按下的键位代码
    /// # 功能
    /// 处理搜索输入、结果选择和跳转
    fn handle_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                // 返回上一个状态
                self.state = self.previous_state.clone();
            }
            KeyCode::Enter => {
                 if let Some(index) = self.selected_search_index {
                     // 跳转到选中的搜索结果
                     if index < self.search_results.len() {
                         let (line_num, _) = self.search_results[index];
                         if let Some(novel) = &mut self.current_novel {
                             novel.progress.scroll_offset = line_num;
                             // 保存进度
                             self.library.update_novel_progress(&novel.path, novel.progress);
                             let _ = self.library.save();
                         }
                         self.state = AppState::Reading;
                     }
                 }
             }
            KeyCode::Up => {
                if !self.search_results.is_empty() {
                    let current = self.selected_search_index.unwrap_or(0);
                    let next = if current > 0 {
                        current - 1
                    } else {
                        self.search_results.len() - 1
                    };
                    self.selected_search_index = Some(next);
                }
            }
            KeyCode::Down => {
                if !self.search_results.is_empty() {
                    if self.selected_search_index.is_none() {
                        self.selected_search_index = Some(0);
                    } else {
                        let current = self.selected_search_index.unwrap();
                        let next = (current + 1) % self.search_results.len();
                        self.selected_search_index = Some(next);
                    }
                }
            }
            KeyCode::Backspace => {
                 // 删除搜索输入的最后一个字符
                 self.search_input.pop();
                 // 自动执行搜索
                 self.perform_search();
             }
            KeyCode::Char(c) => {
                 // 添加字符到搜索输入
                 self.search_input.push(c);
                 // 自动执行搜索（避免中文输入法拼音状态）
                 self.perform_search();
             }
            _ => {}
        }
    }

    /// 执行搜索操作
    /// # 功能
    /// 在当前小说内容中搜索包含关键词的行
    fn perform_search(&mut self) {
        if let Some(novel) = &self.current_novel {
            if !self.search_input.is_empty() {
                let lines: Vec<&str> = novel.content.lines().collect();
                self.search_results.clear();
                
                for (line_num, line) in lines.iter().enumerate() {
                    if line.to_lowercase().contains(&self.search_input.to_lowercase()) {
                        self.search_results.push((line_num, line.to_string()));
                    }
                }
                
                // 更新选中索引，确保不越界
                if !self.search_results.is_empty() {
                    // 如果之前没有选中或选中索引越界，则选中第一个
                    if self.selected_search_index.is_none() 
                        || self.selected_search_index.unwrap() >= self.search_results.len() {
                        self.selected_search_index = Some(0);
                    }
                } else {
                    // 没有搜索结果时清空选中
                    self.selected_search_index = None;
                }
            } else {
                // 搜索输入为空时清空结果
                self.search_results.clear();
                self.selected_search_index = None;
            }
        }
    }
}

fn main() -> Result<()> {
    // 创建并运行应用
    let mut app = App::new().context("创建应用失败")?;
    app.run().context("运行应用失败")?;

    Ok(())
}
