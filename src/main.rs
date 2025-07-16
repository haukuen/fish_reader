mod model;

use anyhow::{Context, Result};
use clap::Command;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
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
    ChapterList,
    Settings,
}

struct App {
    /// 当前应用状态（书架/阅读/搜索/章节目录模式）
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
    /// 当前选中的章节索引
    selected_chapter_index: Option<usize>,
    /// 上一个状态（用于从搜索/章节目录返回）
    previous_state: AppState,
    /// 孤立的小说记录（JSON中存在但文件已删除）
    orphaned_novels: Vec<model::NovelInfo>,
    /// 设置页面中选中的孤立小说索引
    selected_orphaned_index: Option<usize>,
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

        let mut app = App {
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
            selected_chapter_index: None,
            previous_state: AppState::Bookshelf,
            orphaned_novels: Vec::new(),
            selected_orphaned_index: None,
        };

        // 检测孤立的小说记录
        app.detect_orphaned_novels();

        Ok(app)
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
        stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableMouseCapture)?;
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
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            self.handle_key(key.code);
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    _ => {}
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
        stdout()
            .execute(DisableMouseCapture)?
            .execute(LeaveAlternateScreen)?;

        Ok(())
    }

    fn ui(&self, f: &mut Frame) {
        match self.state {
            AppState::Bookshelf => self.render_bookshelf(f),
            AppState::Reading => self.render_reader(f),
            AppState::Searching => self.render_search(f),
            AppState::ChapterList => self.render_chapter_list(f),
            AppState::Settings => self.render_settings(f),
        }
    }

    fn render_bookshelf(&self, f: &mut Frame) {
        let area = f.area();

        // 创建书架标题
        let title = Paragraph::new("书架")
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
        let help_text = "↑/k: 上移  ↓/j: 下移  Enter: 选择  s: 设置  Esc/q: 退出";
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

            // 计算可见内容区域
            let content_area = Rect {
                x: area.x + 1,
                y: area.y,
                width: area.width - 2,
                height: area.height - 1,
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

            // 创建帮助信息（贴近底部）
            let progress_text = format!("进度: {}/{} 行", start_line + 1, lines.len());
            let help_text = format!(
                "{} | ↑/k: 上滚  ↓/j: 下滚  ←/h: 上页  →/l: 下页  /: 搜索  t: 章节目录  Esc: 返回书架  q: 退出",
                progress_text
            );
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
    }

    fn handle_key(&mut self, key: KeyCode) {
        match self.state {
            AppState::Bookshelf => self.handle_bookshelf_key(key),
            AppState::Reading => self.handle_reader_key(key),
            AppState::Searching => self.handle_search_key(key),
            AppState::ChapterList => self.handle_chapter_list_key(key),
            AppState::Settings => self.handle_settings_key(key),
        }
    }

    /// 处理鼠标事件
    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => match self.state {
                AppState::Reading => self.handle_reader_key(KeyCode::Up),
                AppState::Bookshelf => self.handle_bookshelf_key(KeyCode::Up),
                AppState::ChapterList => self.handle_chapter_list_key(KeyCode::Up),
                AppState::Settings => self.handle_settings_key(KeyCode::Up),
                AppState::Searching => self.handle_search_key(KeyCode::Up),
            },
            MouseEventKind::ScrollDown => match self.state {
                AppState::Reading => self.handle_reader_key(KeyCode::Down),
                AppState::Bookshelf => self.handle_bookshelf_key(KeyCode::Down),
                AppState::ChapterList => self.handle_chapter_list_key(KeyCode::Down),
                AppState::Settings => self.handle_settings_key(KeyCode::Down),
                AppState::Searching => self.handle_search_key(KeyCode::Down),
            },
            _ => {}
        }
    }

    fn handle_bookshelf_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
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
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // 进入设置页面
                self.detect_orphaned_novels();
                self.state = AppState::Settings;
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

            let content_height = (self.terminal_size.height as usize)
                .saturating_sub(1) // 帮助信息1行
                .saturating_sub(2) // 上下边框各占1行
                .saturating_sub(1);
            let page_size = content_height.max(1);

            match key {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // 保存阅读进度
                    self.library
                        .update_novel_progress(&novel.path, novel.progress);
                    let _ = self.library.save();
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    // 保存阅读进度并返回书架
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
                    // 向上翻页
                    novel.progress.scroll_offset =
                        novel.progress.scroll_offset.saturating_sub(page_size);
                }
                KeyCode::Right | KeyCode::Char('l') => {
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
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    // 进入章节目录模式
                    self.previous_state = AppState::Reading;
                    self.state = AppState::ChapterList;
                    // 根据当前阅读位置自动选择最接近的章节
                    self.selected_chapter_index = self.find_current_chapter_index();
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

            // 计算滚动偏移，让选中的搜索结果显示在中间位置
            if let Some(selected) = self.selected_search_index {
                let visible_height = results_area.height.saturating_sub(2) as usize; // 减去边框
                let half_height = visible_height / 2;

                if selected >= half_height {
                    let max_offset = self.search_results.len().saturating_sub(visible_height);
                    let offset = (selected.saturating_sub(half_height)).min(max_offset);
                    state = state.with_offset(offset);
                }
            }

            f.render_stateful_widget(results_list, results_area, &mut state);
        }

        // 创建帮助信息
        let help_text = "输入搜索内容 | ↑/↓: 选择结果 | Enter: 跳转 | Esc/q: 返回";
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
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
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
                            self.library
                                .update_novel_progress(&novel.path, novel.progress);
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
                self.perform_search();
            }
            KeyCode::Char(c) => {
                // 添加字符到搜索输入
                self.search_input.push(c);
                self.perform_search();
            }
            _ => {}
        }
    }

    /// 渲染章节目录界面
    /// # 功能
    /// 显示当前小说的章节列表，支持导航和跳转
    fn render_chapter_list(&self, f: &mut Frame) {
        let area = f.area();

        // 创建章节目录标题
        let title = Paragraph::new("章节目录")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 2,
        };

        f.render_widget(title, title_area);

        // 显示章节列表
        if let Some(novel) = &self.current_novel {
            if novel.chapters.is_empty() {
                // 没有检测到章节时显示提示信息
                let no_chapters = Paragraph::new("未检测到章节信息\n\n可能原因：\n• 小说格式不规范\n• 章节标题格式特殊\n• 文件内容较短")
                    .style(Style::default().fg(Color::Yellow))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).title("提示"));

                let content_area = Rect {
                    x: area.x + 2,
                    y: area.y + 2,
                    width: area.width - 4,
                    height: area.height - 5,
                };

                f.render_widget(no_chapters, content_area);
            } else {
                // 创建章节列表
                let items: Vec<ListItem> = novel
                    .chapters
                    .iter()
                    .enumerate()
                    .map(|(index, chapter)| {
                        let prefix = if Some(index) == self.selected_chapter_index {
                            ">> "
                        } else {
                            "   "
                        };
                        let display_text = format!("{}{}", prefix, chapter.title);
                        ListItem::new(display_text).style(Style::default().fg(Color::White))
                    })
                    .collect();

                let chapters_list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("章节列表 (共{}章)", novel.chapters.len())),
                    )
                    .highlight_style(Style::default().bg(Color::DarkGray))
                    .highlight_symbol("");

                let list_area = Rect {
                    x: area.x + 2,
                    y: area.y + 2,
                    width: area.width - 4,
                    height: area.height - 5,
                };

                let mut state = ListState::default();
                state.select(self.selected_chapter_index);

                // 计算滚动偏移，让选中的章节显示在中间位置
                if let Some(selected) = self.selected_chapter_index {
                    let visible_height = list_area.height.saturating_sub(2) as usize; // 减去边框
                    let half_height = visible_height / 2;

                    if selected >= half_height {
                        let max_offset = novel.chapters.len().saturating_sub(visible_height);
                        let offset = (selected.saturating_sub(half_height)).min(max_offset);
                        state = state.with_offset(offset);
                    }
                }

                f.render_stateful_widget(chapters_list, list_area, &mut state);
            }
        }

        // 创建帮助信息
        let help_text = "↑/↓: 选择章节 | Enter: 跳转到章节 | Esc/q: 返回阅读";
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

    /// 执行搜索操作
    /// # 功能
    /// 在当前小说内容中搜索包含关键词的行
    fn perform_search(&mut self) {
        if let Some(novel) = &self.current_novel {
            if !self.search_input.is_empty() {
                let lines: Vec<&str> = novel.content.lines().collect();
                self.search_results.clear();

                for (line_num, line) in lines.iter().enumerate() {
                    if line
                        .to_lowercase()
                        .contains(&self.search_input.to_lowercase())
                    {
                        self.search_results.push((line_num, line.to_string()));
                    }
                }

                // 更新选中索引，确保不越界
                if !self.search_results.is_empty() {
                    // 如果之前没有选中或选中索引越界，则选中第一个
                    if self.selected_search_index.is_none()
                        || self.selected_search_index.unwrap() >= self.search_results.len()
                    {
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

    /// 根据当前阅读位置找到最接近的章节索引
    /// # 返回
    /// 返回最接近当前阅读位置的章节索引，如果没有章节则返回None
    fn find_current_chapter_index(&self) -> Option<usize> {
        if let Some(novel) = &self.current_novel {
            if novel.chapters.is_empty() {
                return None;
            }

            let current_line = novel.progress.scroll_offset;
            let mut best_index = 0;

            // 找到当前阅读位置之前的最后一个章节
            for (index, chapter) in novel.chapters.iter().enumerate() {
                if chapter.start_line <= current_line {
                    best_index = index;
                } else {
                    break;
                }
            }

            Some(best_index)
        } else {
            None
        }
    }

    /// 处理章节目录模式下的键盘事件
    /// # 参数
    /// - `key`: 按下的键位代码
    /// # 功能
    /// 处理章节选择、跳转和返回
    fn handle_chapter_list_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                // 返回阅读模式
                self.state = self.previous_state.clone();
            }
            KeyCode::Enter => {
                if let Some(index) = self.selected_chapter_index {
                    if let Some(novel) = &mut self.current_novel {
                        if index < novel.chapters.len() {
                            // 跳转到选中的章节
                            let chapter = &novel.chapters[index];
                            novel.progress.scroll_offset = chapter.start_line;
                            // 保存进度
                            self.library
                                .update_novel_progress(&novel.path, novel.progress);
                            let _ = self.library.save();
                            self.state = AppState::Reading;
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(novel) = &self.current_novel {
                    if !novel.chapters.is_empty() {
                        let current = self.selected_chapter_index.unwrap_or(0);
                        let next = if current > 0 {
                            current - 1
                        } else {
                            novel.chapters.len() - 1
                        };
                        self.selected_chapter_index = Some(next);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(novel) = &self.current_novel {
                    if !novel.chapters.is_empty() {
                        if self.selected_chapter_index.is_none() {
                            self.selected_chapter_index = Some(0);
                        } else {
                            let current = self.selected_chapter_index.unwrap();
                            let next = (current + 1) % novel.chapters.len();
                            self.selected_chapter_index = Some(next);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// 检测孤立的小说记录（JSON中存在但文件已删除）
    /// # 功能
    /// 扫描library中的所有小说记录，找出文件已被删除的记录
    fn detect_orphaned_novels(&mut self) {
        self.orphaned_novels.clear();

        for novel_info in &self.library.novels {
            if !novel_info.path.exists() {
                self.orphaned_novels.push(novel_info.clone());
            }
        }

        // 重置选中索引
        self.selected_orphaned_index = None;
    }

    /// 渲染设置页面界面
    /// # 功能
    /// 显示孤立的小说记录列表，允许用户选择删除
    fn render_settings(&self, f: &mut Frame) {
        let area = f.area();

        // 创建设置页面标题
        let title = Paragraph::new("设置 - 清理孤立记录")
            .style(Style::default().fg(Color::Magenta))
            .alignment(Alignment::Center);

        let title_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 2,
        };

        f.render_widget(title, title_area);

        if self.orphaned_novels.is_empty() {
            // 没有孤立记录时显示提示信息
            let no_orphaned = Paragraph::new("没有发现孤立的小说记录")
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("状态"));

            let content_area = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width - 4,
                height: area.height - 5,
            };

            f.render_widget(no_orphaned, content_area);
        } else {
            // 显示孤立记录列表
            let items: Vec<ListItem> = self
                .orphaned_novels
                .iter()
                .enumerate()
                .map(|(index, novel_info)| {
                    let prefix = if Some(index) == self.selected_orphaned_index {
                        ">> "
                    } else {
                        "   "
                    };
                    let display_text = format!(
                        "{}{} ({})",
                        prefix,
                        novel_info.title,
                        novel_info.path.display()
                    );
                    ListItem::new(display_text).style(Style::default().fg(Color::Yellow))
                })
                .collect();

            let orphaned_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("孤立记录 (共{}条)", self.orphaned_novels.len())),
                )
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("");

            let list_area = Rect {
                x: area.x + 2,
                y: area.y + 2,
                width: area.width - 4,
                height: area.height - 5,
            };

            let mut state = ListState::default();
            state.select(self.selected_orphaned_index);

            f.render_stateful_widget(orphaned_list, list_area, &mut state);
        }

        // 创建帮助信息
        let help_text = if self.orphaned_novels.is_empty() {
            "Esc/q: 返回书架"
        } else {
            "↑/↓: 选择记录 | D/d: 删除选中记录 | Esc/q: 返回书架"
        };
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

    /// 处理设置页面的键盘事件
    /// # 参数
    /// - `key`: 按下的键位代码
    /// # 功能
    /// 处理孤立记录的选择和删除操作
    fn handle_settings_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                // 返回书架
                self.state = AppState::Bookshelf;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.orphaned_novels.is_empty() {
                    let current = self.selected_orphaned_index.unwrap_or(0);
                    let next = if current > 0 {
                        current - 1
                    } else {
                        self.orphaned_novels.len() - 1
                    };
                    self.selected_orphaned_index = Some(next);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.orphaned_novels.is_empty() {
                    if self.selected_orphaned_index.is_none() {
                        self.selected_orphaned_index = Some(0);
                    } else {
                        let current = self.selected_orphaned_index.unwrap();
                        let next = (current + 1) % self.orphaned_novels.len();
                        self.selected_orphaned_index = Some(next);
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // 删除选中的孤立记录
                if let Some(index) = self.selected_orphaned_index {
                    if index < self.orphaned_novels.len() {
                        let orphaned_novel = &self.orphaned_novels[index];

                        self.library
                            .novels
                            .retain(|n| n.path != orphaned_novel.path);

                        let _ = self.library.save();

                        self.detect_orphaned_novels();

                        // 调整选中索引
                        if !self.orphaned_novels.is_empty() {
                            let new_index = index.min(self.orphaned_novels.len() - 1);
                            self.selected_orphaned_index = Some(new_index);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let _matches = Command::new("fish_reader")
        .version(env!("CARGO_PKG_VERSION"))
        .author("haukuen")
        .about("A terminal-based novel reader with bookshelf management")
        .get_matches();

    let mut app = App::new().context("创建应用失败")?;
    app.run().context("运行应用失败")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::fs;
    use tempfile::tempdir;

    /// 测试应用状态枚举的克隆功能
    #[test]
    fn test_app_state_clone() {
        let state = AppState::Bookshelf;
        let cloned_state = state.clone();

        // 由于AppState没有实现PartialEq，通过模式匹配来验证
        match (state, cloned_state) {
            (AppState::Bookshelf, AppState::Bookshelf) => {}
            _ => panic!("AppState clone failed"),
        }
    }

    /// 测试应用初始化功能
    #[test]
    fn test_app_new() -> Result<()> {
        let app = App::new()?;

        // 验证初始状态
        assert!(matches!(app.state, AppState::Bookshelf));
        assert_eq!(app.selected_novel_index, None);
        assert_eq!(app.current_novel, None);
        assert_eq!(app.should_quit, false);
        assert_eq!(app.search_input, "");
        assert!(app.search_results.is_empty());
        assert_eq!(app.selected_search_index, None);

        Ok(())
    }

    /// 测试小说目录获取功能
    #[test]
    fn test_get_novels_dir() {
        let novels_dir = App::get_novels_dir();

        // 验证路径包含正确的目录结构
        assert!(novels_dir.to_string_lossy().contains(".fish_reader"));
        assert!(novels_dir.to_string_lossy().contains("novels"));
    }

    /// 测试从目录加载小说功能
    #[test]
    fn test_load_novels_from_dir() -> std::io::Result<()> {
        let dir = tempdir()?;

        // 创建测试小说文件
        let novel1_path = dir.path().join("novel1.txt");
        let novel2_path = dir.path().join("novel2.txt");
        let non_txt_path = dir.path().join("readme.md");

        fs::write(&novel1_path, "第一章\n这是第一本小说的内容")?;
        fs::write(&novel2_path, "序章\n这是第二本小说的内容")?;
        fs::write(&non_txt_path, "这不是小说文件")?;

        let novels = App::load_novels_from_dir(dir.path()).unwrap();

        // 验证只加载了txt文件
        assert_eq!(novels.len(), 2);

        // 验证小说标题正确提取
        let titles: Vec<&str> = novels.iter().map(|n| n.title.as_str()).collect();
        assert!(titles.contains(&"novel1"));
        assert!(titles.contains(&"novel2"));

        // 验证内容正确加载
        let novel1 = novels.iter().find(|n| n.title == "novel1").unwrap();
        assert!(novel1.content.contains("第一章"));

        Ok(())
    }

    /// 测试从不存在的目录加载小说
    #[test]
    fn test_load_novels_from_nonexistent_dir() {
        let non_existent_path = PathBuf::from("/tmp/non_existent_novels_dir_12345");
        let novels = App::load_novels_from_dir(&non_existent_path).unwrap();

        // 应该返回空的小说列表
        assert!(novels.is_empty());
    }

    /// 测试书架模式下的退出逻辑
    #[test]
    fn test_bookshelf_quit_logic() -> Result<()> {
        let mut app = App::new()?;

        // 测试各种退出键
        let quit_keys = [KeyCode::Char('q'), KeyCode::Char('Q'), KeyCode::Esc];

        for key in quit_keys {
            app.should_quit = false;
            app.handle_bookshelf_key(key);
            assert!(app.should_quit, "Key {:?} should trigger quit", key);
        }

        Ok(())
    }

    /// 测试书架导航状态变化逻辑
    #[test]
    fn test_bookshelf_navigation_logic() -> Result<()> {
        let mut app = App::new()?;

        // 创建测试数据
        app.novels = vec![
            Novel::new(PathBuf::from("novel1.txt")),
            Novel::new(PathBuf::from("novel2.txt")),
            Novel::new(PathBuf::from("novel3.txt")),
        ];

        // 测试初始状态
        assert_eq!(app.selected_novel_index, None);

        // 测试向下导航逻辑
        let down_keys = [KeyCode::Down, KeyCode::Char('j')];
        for key in down_keys {
            app.selected_novel_index = None;
            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(0),
                "First down should select index 0"
            );

            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(1),
                "Second down should select index 1"
            );

            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(2),
                "Third down should select index 2"
            );

            // 测试循环
            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(0),
                "Fourth down should wrap to index 0"
            );
        }

        // 测试向上导航逻辑
        let up_keys = [KeyCode::Up, KeyCode::Char('k')];
        for key in up_keys {
            app.selected_novel_index = Some(0);
            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(2),
                "Up from 0 should wrap to 2"
            );

            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(1),
                "Up from 2 should go to 1"
            );

            app.handle_bookshelf_key(key);
            assert_eq!(
                app.selected_novel_index,
                Some(0),
                "Up from 1 should go to 0"
            );
        }

        Ok(())
    }

    /// 测试书架选择小说逻辑
    #[test]
    fn test_bookshelf_selection_logic() -> Result<()> {
        let mut app = App::new()?;

        // 创建测试数据
        app.novels = vec![Novel::new(PathBuf::from("test_novel.txt"))];
        app.selected_novel_index = Some(0);

        // 测试选择小说
        app.handle_bookshelf_key(KeyCode::Enter);

        // 验证状态变化
        assert!(matches!(app.state, AppState::Reading));
        assert!(app.current_novel.is_some());

        Ok(())
    }

    /// 测试书架设置页面跳转逻辑
    #[test]
    fn test_bookshelf_settings_transition() -> Result<()> {
        let mut app = App::new()?;

        let settings_keys = [KeyCode::Char('s'), KeyCode::Char('S')];

        for key in settings_keys {
            app.state = AppState::Bookshelf;
            app.handle_bookshelf_key(key);
            assert!(
                matches!(app.state, AppState::Settings),
                "Key {:?} should go to settings",
                key
            );
        }

        Ok(())
    }

    /// 测试空书架的导航
    #[test]
    fn test_empty_bookshelf_navigation() -> Result<()> {
        let mut app = App::new()?;

        // 确保小说列表为空
        app.novels.clear();

        // 测试在空列表中导航不会崩溃
        app.handle_bookshelf_key(KeyCode::Down);
        assert_eq!(app.selected_novel_index, None);

        app.handle_bookshelf_key(KeyCode::Up);
        assert_eq!(app.selected_novel_index, None);

        Ok(())
    }

    /// 测试搜索算法逻辑
    #[test]
    fn test_search_algorithm_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试小说内容
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "第一章 开始\n第二章 发展\n第三章 高潮\n第四章 结局".to_string();
        app.current_novel = Some(novel);

        // 测试通用搜索
        app.search_input = "第".to_string();
        app.perform_search();
        assert_eq!(
            app.search_results.len(),
            4,
            "Should find 4 lines containing '第'"
        );
        assert_eq!(
            app.selected_search_index,
            Some(0),
            "Should auto-select first result"
        );

        // 测试精确搜索
        app.search_input = "高潮".to_string();
        app.perform_search();
        assert_eq!(
            app.search_results.len(),
            1,
            "Should find 1 line containing '高潮'"
        );
        assert_eq!(app.search_results[0].0, 2, "Should be line index 2");
        assert!(
            app.search_results[0].1.contains("高潮"),
            "Result should contain search term"
        );

        // 测试大小写不敏感搜索
        app.search_input = "开始".to_string();
        app.perform_search();
        assert_eq!(
            app.search_results.len(),
            1,
            "Should find case-insensitive match"
        );

        // 测试无结果搜索
        app.search_input = "不存在的内容".to_string();
        app.perform_search();
        assert!(app.search_results.is_empty(), "Should return empty results");
        assert_eq!(app.selected_search_index, None, "Should clear selection");

        // 测试空搜索
        app.search_input.clear();
        app.perform_search();
        assert!(
            app.search_results.is_empty(),
            "Empty search should clear results"
        );
        assert_eq!(
            app.selected_search_index, None,
            "Empty search should clear selection"
        );

        Ok(())
    }

    /// 测试搜索输入处理逻辑
    #[test]
    fn test_search_input_logic() -> Result<()> {
        let mut app = App::new()?;
        app.state = AppState::Searching;
        app.previous_state = AppState::Reading;

        // 测试字符输入
        app.handle_search_key(KeyCode::Char('测'));
        assert_eq!(
            app.search_input, "测",
            "Should add character to search input"
        );

        app.handle_search_key(KeyCode::Char('试'));
        assert_eq!(
            app.search_input, "测试",
            "Should append character to search input"
        );

        // 测试退格键
        app.handle_search_key(KeyCode::Backspace);
        assert_eq!(
            app.search_input, "测",
            "Backspace should remove last character"
        );

        app.handle_search_key(KeyCode::Backspace);
        assert_eq!(
            app.search_input, "",
            "Backspace should remove all characters"
        );

        // 测试空字符串退格
        app.handle_search_key(KeyCode::Backspace);
        assert_eq!(
            app.search_input, "",
            "Backspace on empty string should not crash"
        );

        Ok(())
    }

    /// 测试搜索状态转换逻辑
    #[test]
    fn test_search_state_transitions() -> Result<()> {
        let mut app = App::new()?;
        app.state = AppState::Searching;
        app.previous_state = AppState::Reading;

        // 测试退出键
        let exit_keys = [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('Q')];
        for key in exit_keys {
            app.state = AppState::Searching;
            app.handle_search_key(key);
            assert!(
                matches!(app.state, AppState::Reading),
                "Key {:?} should return to previous state",
                key
            );
        }

        Ok(())
    }

    /// 测试搜索结果导航逻辑
    #[test]
    fn test_search_navigation_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试搜索结果
        app.search_results = vec![
            (0, "第一行".to_string()),
            (2, "第三行".to_string()),
            (4, "第五行".to_string()),
        ];
        app.selected_search_index = None;

        // 测试向下导航
        app.handle_search_key(KeyCode::Down);
        assert_eq!(
            app.selected_search_index,
            Some(0),
            "First down should select index 0"
        );

        app.handle_search_key(KeyCode::Down);
        assert_eq!(
            app.selected_search_index,
            Some(1),
            "Second down should select index 1"
        );

        app.handle_search_key(KeyCode::Down);
        assert_eq!(
            app.selected_search_index,
            Some(2),
            "Third down should select index 2"
        );

        // 测试循环
        app.handle_search_key(KeyCode::Down);
        assert_eq!(
            app.selected_search_index,
            Some(0),
            "Fourth down should wrap to index 0"
        );

        // 测试向上导航
        app.handle_search_key(KeyCode::Up);
        assert_eq!(
            app.selected_search_index,
            Some(2),
            "Up should wrap to last index"
        );

        app.handle_search_key(KeyCode::Up);
        assert_eq!(
            app.selected_search_index,
            Some(1),
            "Up should go to previous index"
        );

        Ok(())
    }

    /// 测试搜索结果跳转逻辑
    #[test]
    fn test_search_jump_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "line1\nline2\nline3".to_string();
        app.current_novel = Some(novel);
        app.state = AppState::Searching;

        app.search_results = vec![(1, "line2".to_string())];
        app.selected_search_index = Some(0);

        // 测试跳转
        app.handle_search_key(KeyCode::Enter);

        // 验证跳转结果
        assert!(
            matches!(app.state, AppState::Reading),
            "Should return to reading state"
        );
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            1,
            "Should jump to correct line"
        );

        Ok(())
    }

    /// 测试阅读器滚动逻辑
    #[test]
    fn test_reader_scroll_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试小说
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = (0..10)
            .map(|i| format!("第{}行内容", i))
            .collect::<Vec<_>>()
            .join("\n");
        app.current_novel = Some(novel);
        app.state = AppState::Reading;

        // 测试向下滚动键
        let down_keys = [KeyCode::Down, KeyCode::Char('j')];
        for key in down_keys {
            app.current_novel.as_mut().unwrap().progress.scroll_offset = 0;
            app.handle_reader_key(key);
            assert_eq!(
                app.current_novel.as_ref().unwrap().progress.scroll_offset,
                1,
                "Key {:?} should scroll down by 1",
                key
            );
        }

        // 测试向上滚动键
        let up_keys = [KeyCode::Up, KeyCode::Char('k')];
        for key in up_keys {
            app.current_novel.as_mut().unwrap().progress.scroll_offset = 2;
            app.handle_reader_key(key);
            assert_eq!(
                app.current_novel.as_ref().unwrap().progress.scroll_offset,
                1,
                "Key {:?} should scroll up by 1",
                key
            );
        }

        // 测试边界条件 - 不能滚动到负数
        app.current_novel.as_mut().unwrap().progress.scroll_offset = 0;
        app.handle_reader_key(KeyCode::Up);
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            0,
            "Should not scroll below 0"
        );

        Ok(())
    }

    /// 测试阅读器翻页逻辑
    #[test]
    fn test_reader_paging_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置长文本小说和终端尺寸
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = (0..100)
            .map(|i| format!("第{}行内容", i))
            .collect::<Vec<_>>()
            .join("\n");
        app.current_novel = Some(novel);
        app.terminal_size = Rect::new(0, 0, 80, 24);
        app.state = AppState::Reading;

        // 测试向下翻页键
        let page_down_keys = [KeyCode::Right, KeyCode::Char('l')];
        for key in page_down_keys {
            app.current_novel.as_mut().unwrap().progress.scroll_offset = 0;
            app.handle_reader_key(key);
            let offset = app.current_novel.as_ref().unwrap().progress.scroll_offset;
            assert!(offset > 0, "Key {:?} should page down", key);
        }

        // 测试向上翻页键
        let page_up_keys = [KeyCode::Left, KeyCode::Char('h')];
        for key in page_up_keys {
            app.current_novel.as_mut().unwrap().progress.scroll_offset = 20;
            let initial_offset = app.current_novel.as_ref().unwrap().progress.scroll_offset;
            app.handle_reader_key(key);
            let final_offset = app.current_novel.as_ref().unwrap().progress.scroll_offset;
            assert!(
                final_offset < initial_offset,
                "Key {:?} should page up",
                key
            );
        }

        // 测试翻页边界条件
        app.current_novel.as_mut().unwrap().progress.scroll_offset = 0;
        app.handle_reader_key(KeyCode::Left);
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            0,
            "Should not page up below 0"
        );

        Ok(())
    }

    /// 测试阅读器状态转换逻辑
    #[test]
    fn test_reader_state_transitions() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "测试内容".to_string();
        app.current_novel = Some(novel);
        app.state = AppState::Reading;

        // 测试搜索模式切换
        app.handle_reader_key(KeyCode::Char('/'));
        assert!(
            matches!(app.state, AppState::Searching),
            "'/' should switch to search mode"
        );
        assert!(
            matches!(app.previous_state, AppState::Reading),
            "Previous state should be Reading"
        );
        assert!(
            app.search_input.is_empty(),
            "Search input should be cleared"
        );
        assert!(
            app.search_results.is_empty(),
            "Search results should be cleared"
        );

        // 测试返回书架
        app.state = AppState::Reading;
        app.handle_reader_key(KeyCode::Esc);
        assert!(
            matches!(app.state, AppState::Bookshelf),
            "Esc should return to bookshelf"
        );

        // 测试退出应用
        app.state = AppState::Reading;
        app.should_quit = false;
        app.handle_reader_key(KeyCode::Char('q'));
        assert!(app.should_quit, "'q' should set quit flag");

        Ok(())
    }

    /// 测试阅读器章节列表切换逻辑
    #[test]
    fn test_reader_chapter_list_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "第一章\n内容1\n第二章\n内容2".to_string();
        app.current_novel = Some(novel);
        app.state = AppState::Reading;

        // 测试章节列表切换
        app.handle_reader_key(KeyCode::Char('t'));
        assert!(
            matches!(app.state, AppState::ChapterList),
            "'t' should switch to chapter list"
        );

        Ok(())
    }

    /// 测试章节列表导航逻辑
    #[test]
    fn test_chapter_list_navigation_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "第一章\n内容1\n第二章\n内容2\n第三章\n内容3".to_string();
        novel.parse_chapters(); // 解析章节
        app.current_novel = Some(novel);
        app.state = AppState::ChapterList;
        app.selected_chapter_index = None;

        // 测试向下导航
        app.handle_chapter_list_key(KeyCode::Down);
        assert_eq!(
            app.selected_chapter_index,
            Some(0),
            "First down should select index 0"
        );

        app.handle_chapter_list_key(KeyCode::Down);
        assert_eq!(
            app.selected_chapter_index,
            Some(1),
            "Second down should select index 1"
        );

        app.handle_chapter_list_key(KeyCode::Down);
        assert_eq!(
            app.selected_chapter_index,
            Some(2),
            "Third down should select index 2"
        );

        // 测试循环
        app.handle_chapter_list_key(KeyCode::Down);
        assert_eq!(
            app.selected_chapter_index,
            Some(0),
            "Fourth down should wrap to index 0"
        );

        // 测试向上导航
        app.handle_chapter_list_key(KeyCode::Up);
        assert_eq!(
            app.selected_chapter_index,
            Some(2),
            "Up should wrap to last index"
        );

        app.handle_chapter_list_key(KeyCode::Up);
        assert_eq!(
            app.selected_chapter_index,
            Some(1),
            "Up should go to previous index"
        );

        Ok(())
    }

    /// 测试章节列表状态转换逻辑
    #[test]
    fn test_chapter_list_state_transitions() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "第一章\n内容1\n第二章\n内容2".to_string();
        novel.parse_chapters(); // 解析章节
        app.current_novel = Some(novel);
        app.state = AppState::ChapterList;
        app.previous_state = AppState::Reading; // 设置前一个状态

        // 测试退出键
        let exit_keys = [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('Q')];
        for key in exit_keys {
            app.state = AppState::ChapterList;
            app.handle_chapter_list_key(key);
            assert!(
                matches!(app.state, AppState::Reading),
                "Key {:?} should return to reading",
                key
            );
        }

        Ok(())
    }

    /// 测试章节跳转逻辑
    #[test]
    fn test_chapter_jump_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        let mut novel = Novel::new(PathBuf::from("test.txt"));
        novel.content = "第一章\n内容1\n第二章\n内容2\n第三章\n内容3".to_string();
        novel.parse_chapters(); // 解析章节
        app.current_novel = Some(novel);
        app.state = AppState::ChapterList;
        app.selected_chapter_index = Some(1); // 选择第二章

        // 测试跳转
        app.handle_chapter_list_key(KeyCode::Enter);

        // 验证跳转结果
        assert!(
            matches!(app.state, AppState::Reading),
            "Should return to reading state"
        );
        assert_eq!(
            app.current_novel.as_ref().unwrap().progress.scroll_offset,
            2,
            "Should jump to chapter 2 line"
        );

        Ok(())
    }

    /// 测试设置页面导航逻辑
    #[test]
    fn test_settings_navigation_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        app.state = AppState::Settings;
        app.orphaned_novels = vec![
            model::NovelInfo {
                title: "小说1".to_string(),
                path: PathBuf::from("novel1.txt"),
                progress: model::ReadingProgress::default(),
            },
            model::NovelInfo {
                title: "小说2".to_string(),
                path: PathBuf::from("novel2.txt"),
                progress: model::ReadingProgress::default(),
            },
            model::NovelInfo {
                title: "小说3".to_string(),
                path: PathBuf::from("novel3.txt"),
                progress: model::ReadingProgress::default(),
            },
        ];
        app.selected_orphaned_index = None;

        // 测试向下导航
        app.handle_settings_key(KeyCode::Down);
        assert_eq!(
            app.selected_orphaned_index,
            Some(0),
            "First down should select index 0"
        );

        app.handle_settings_key(KeyCode::Down);
        assert_eq!(
            app.selected_orphaned_index,
            Some(1),
            "Second down should select index 1"
        );

        app.handle_settings_key(KeyCode::Down);
        assert_eq!(
            app.selected_orphaned_index,
            Some(2),
            "Third down should select index 2"
        );

        // 测试循环
        app.handle_settings_key(KeyCode::Down);
        assert_eq!(
            app.selected_orphaned_index,
            Some(0),
            "Fourth down should wrap to index 0"
        );

        // 测试向上导航
        app.handle_settings_key(KeyCode::Up);
        assert_eq!(
            app.selected_orphaned_index,
            Some(2),
            "Up should wrap to last index"
        );

        app.handle_settings_key(KeyCode::Up);
        assert_eq!(
            app.selected_orphaned_index,
            Some(1),
            "Up should go to previous index"
        );

        Ok(())
    }

    /// 测试设置页面状态转换逻辑
    #[test]
    fn test_settings_state_transitions() -> Result<()> {
        let mut app = App::new()?;
        app.state = AppState::Settings;

        // 测试退出键
        let exit_keys = [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('Q')];
        for key in exit_keys {
            app.state = AppState::Settings;
            app.handle_settings_key(key);
            assert!(
                matches!(app.state, AppState::Bookshelf),
                "Key {:?} should return to bookshelf",
                key
            );
        }

        Ok(())
    }

    /// 测试设置页面删除逻辑
    #[test]
    fn test_settings_delete_logic() -> Result<()> {
        let mut app = App::new()?;

        // 设置测试环境
        app.state = AppState::Settings;

        // 添加测试数据到 library
        app.library.novels = vec![
            model::NovelInfo {
                title: "小说1".to_string(),
                path: PathBuf::from("novel1.txt"),
                progress: model::ReadingProgress::default(),
            },
            model::NovelInfo {
                title: "小说2".to_string(),
                path: PathBuf::from("novel2.txt"),
                progress: model::ReadingProgress::default(),
            },
        ];

        app.orphaned_novels = vec![
            model::NovelInfo {
                title: "小说1".to_string(),
                path: PathBuf::from("novel1.txt"),
                progress: model::ReadingProgress::default(),
            },
            model::NovelInfo {
                title: "小说2".to_string(),
                path: PathBuf::from("novel2.txt"),
                progress: model::ReadingProgress::default(),
            },
        ];
        app.selected_orphaned_index = Some(0);

        // 测试删除操作
        let delete_keys = [KeyCode::Char('d'), KeyCode::Char('D')];
        for key in delete_keys {
            app.orphaned_novels = vec![
                model::NovelInfo {
                    title: "小说1".to_string(),
                    path: PathBuf::from("novel1.txt"),
                    progress: model::ReadingProgress::default(),
                },
                model::NovelInfo {
                    title: "小说2".to_string(),
                    path: PathBuf::from("novel2.txt"),
                    progress: model::ReadingProgress::default(),
                },
            ];
            app.selected_orphaned_index = Some(0);
            let initial_count = app.orphaned_novels.len();

            app.handle_settings_key(key);

            assert_eq!(
                app.orphaned_novels.len(),
                initial_count - 1,
                "Key {:?} should delete selected item",
                key
            );
            assert_eq!(
                app.selected_orphaned_index,
                Some(0),
                "Selection should adjust after deletion"
            );
        }

        Ok(())
    }

    // 事件-状态机层测试（使用 TestBackend）

    /// 测试书架界面渲染
    #[test]
    fn test_bookshelf_rendering() -> Result<()> {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new()?;

        // 设置测试数据
        app.novels = vec![
            Novel::new(PathBuf::from("test1.txt")),
            Novel::new(PathBuf::from("test2.txt")),
        ];
        app.selected_novel_index = Some(0);
        app.state = AppState::Bookshelf;

        // 渲染一帧
        terminal.draw(|f| app.ui(f))?;

        // 验证渲染没有崩溃
        Ok(())
    }

    /// 测试搜索界面渲染
    #[test]
    fn test_search_rendering() -> Result<()> {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new()?;

        // 设置测试数据
        app.state = AppState::Searching;
        app.search_input = "测试".to_string();
        app.search_results = vec![
            (0, "第一行测试内容".to_string()),
            (2, "第三行测试内容".to_string()),
        ];
        app.selected_search_index = Some(0);

        // 渲染一帧
        terminal.draw(|f| app.ui(f))?;

        // 验证渲染没有崩溃
        Ok(())
    }
}
