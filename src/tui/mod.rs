use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap
    },
    Frame, Terminal,
};
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use crate::api::{BoardClient, DeviceCode, Paste};
use crate::config::AppConfig;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub enum AppMode {
    Main,
    CreatePaste,
    ViewPaste,
    EnterDeviceCode,
    Help,
    Error,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Idle,
    Loading(String),  // Loading with message
}

#[derive(Debug)]
pub enum AsyncResult {
    DeviceRegistered(BoardClient, DeviceCode, AppConfig),
    PastesLoaded(Vec<Paste>),
    PasteCreated(Paste),
    CustomDeviceSet(BoardClient, DeviceCode, AppConfig),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub should_quit: bool,
    pub client: Option<BoardClient>,
    pub device_code: Option<DeviceCode>,
    pub config: AppConfig,
    pub pastes: Vec<Paste>,
    pub selected_paste: Option<usize>,
    pub input_buffer: String,
    pub status_message: String,
    pub error_message: Option<String>,
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
    pub view_scroll: usize,
    pub loading_state: LoadingState,
    pub loading_start: Option<Instant>,
}

pub struct App {
    state: AppState,
    runtime: Runtime,
    async_receiver: mpsc::Receiver<AsyncResult>,
    async_sender: mpsc::Sender<AsyncResult>,
}

impl App {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()?;
        let (async_sender, async_receiver) = mpsc::channel();

        // Load configuration
        let config = AppConfig::load()?;

        let state = AppState {
            mode: AppMode::Main,
            should_quit: false,
            client: None,
            device_code: None,
            config,
            pastes: Vec::new(),
            selected_paste: None,
            input_buffer: String::new(),
            status_message: "Welcome to Board TUI! Press 'h' for help".to_string(),
            error_message: None,
            list_state: ListState::default(),
            scroll_state: ScrollbarState::new(0),
            view_scroll: 0,
            loading_state: LoadingState::Loading("Initializing API client...".to_string()),
            loading_start: Some(Instant::now()),
        };

        // Initialize API client asynchronously
        let sender = async_sender.clone();
        runtime.spawn(async move {
            match Self::initialize_client().await {
                Ok((client, device_code, config)) => {
                    let _ = sender.send(AsyncResult::DeviceRegistered(client, device_code, config));
                }
                Err(e) => {
                    let _ = sender.send(AsyncResult::Error(format!("Failed to initialize API client: {}", e)));
                }
            }
        });

        Ok(Self {
            state,
            runtime,
            async_receiver,
            async_sender,
        })
    }

    async fn initialize_client() -> Result<(BoardClient, DeviceCode, AppConfig)> {
        let mut config = AppConfig::load()?;
        let mut client = BoardClient::new()?;

        let device_code = if let Some(config_device_code) = config.get_device_code() {
            // Use device code from config
            client.set_device_code(config_device_code.clone());
            config_device_code
        } else if let Ok(env_device_code) = std::env::var("BOARD_DEVICE_CODE") {
            // Migrate from environment variable to config
            let device_code = DeviceCode::from(env_device_code);
            config.set_device_code(device_code.clone())?;
            client.set_device_code(device_code.clone());
            device_code
        } else {
            // Register new device and save to config
            let device_code = client.register_device().await?;
            config.set_device_code(device_code.clone())?;
            device_code
        };

        Ok((client, device_code, config))
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the app loop
        while !self.state.should_quit {
            terminal.draw(|f| self.ui(f))?;

            // Check for async results first
            if let Ok(result) = self.async_receiver.try_recv() {
                self.handle_async_result(result);
            }

            // Handle events with timeout for loading animations
            let timeout = match self.state.loading_state {
                LoadingState::Loading(_) => Duration::from_millis(100), // Fast refresh for loading
                LoadingState::Idle => Duration::from_millis(250),       // Slower when idle
            };

            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key.code, key.modifiers);
                }
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn handle_async_result(&mut self, result: AsyncResult) {
        match result {
            AsyncResult::DeviceRegistered(client, device_code, config) => {
                self.state.client = Some(client);
                self.state.device_code = Some(device_code.clone());
                self.state.config = config;
                self.state.status_message = format!("Connected with device: {}", device_code);
                self.stop_loading();
                self.refresh_pastes();
            }
            AsyncResult::CustomDeviceSet(client, device_code, config) => {
                self.state.client = Some(client);
                self.state.device_code = Some(device_code.clone());
                self.state.config = config;
                self.state.status_message = format!("Connected with custom device: {}", device_code);
                self.state.mode = AppMode::Main;
                self.state.input_buffer.clear();
                self.stop_loading();
                self.refresh_pastes();
            }
            AsyncResult::PastesLoaded(pastes) => {
                self.state.pastes = pastes;
                self.state.status_message = format!("Loaded {} pastes", self.state.pastes.len());
                if !self.state.pastes.is_empty() && self.state.selected_paste.is_none() {
                    self.state.selected_paste = Some(0);
                    self.state.list_state.select(Some(0));
                }
                self.update_scroll_state();
                self.stop_loading();
            }
            AsyncResult::PasteCreated(paste) => {
                self.state.pastes.insert(0, paste.clone());
                self.state.status_message = format!("Created paste: {}", paste.id);
                self.state.mode = AppMode::Main;
                self.state.input_buffer.clear();
                self.state.selected_paste = Some(0);
                self.state.list_state.select(Some(0));
                self.update_scroll_state();
                self.stop_loading();
            }
            AsyncResult::Error(error) => {
                self.state.status_message = error;
                self.stop_loading();
            }
        }
    }

    fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        // Don't handle input during loading (except quit)
        if matches!(self.state.loading_state, LoadingState::Loading(_)) {
            match key {
                KeyCode::Char('q') | KeyCode::Esc => self.state.should_quit = true,
                _ => return,
            }
        }

        match self.state.mode {
            AppMode::Main => self.handle_main_input(key, modifiers),
            AppMode::CreatePaste => self.handle_create_input(key, modifiers),
            AppMode::ViewPaste => self.handle_view_input(key),
            AppMode::EnterDeviceCode => self.handle_device_code_input(key, modifiers),
            AppMode::Help => self.handle_help_input(key),
            AppMode::Error => self.handle_error_input(key),
        }
    }

    fn handle_main_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.state.should_quit = true,
            KeyCode::Char('h') => self.state.mode = AppMode::Help,
            KeyCode::Char('c') => {
                self.state.mode = AppMode::CreatePaste;
                self.state.input_buffer.clear();
                self.state.status_message = "Enter paste content (Enter to save, Esc to cancel)".to_string();
            },
            KeyCode::Char('r') => self.refresh_pastes(),
            KeyCode::Char('n') => self.register_new_device(),
            KeyCode::Char('e') => {
                self.state.mode = AppMode::EnterDeviceCode;
                self.state.input_buffer.clear();
                self.state.status_message = "Enter 8-character device code (Esc to cancel)".to_string();
            },
            KeyCode::Up | KeyCode::Char('k') => self.previous_paste(),
            KeyCode::Down | KeyCode::Char('j') => self.next_paste(),
            KeyCode::Enter => self.view_selected_paste(),
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(selected) = self.state.selected_paste {
                    if selected < self.state.pastes.len() {
                        let paste = &self.state.pastes[selected];
                        self.copy_paste_url(paste.url.clone());
                    }
                }
            },
            _ => {}
        }
    }

    fn handle_create_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match key {
            KeyCode::Esc => {
                self.state.mode = AppMode::Main;
                self.state.input_buffer.clear();
                self.state.status_message = "Paste creation cancelled".to_string();
            },
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.create_paste();
            },
            KeyCode::Char(c) => {
                self.state.input_buffer.push(c);
            },
            KeyCode::Backspace => {
                self.state.input_buffer.pop();
            },
            KeyCode::Enter => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.state.input_buffer.push('\n');
                } else {
                    self.create_paste();
                }
            },
            _ => {}
        }
    }

    fn handle_device_code_input(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match key {
            KeyCode::Esc => {
                self.state.mode = AppMode::Main;
                self.state.input_buffer.clear();
                self.state.status_message = "Device code entry cancelled".to_string();
            },
            KeyCode::Enter => {
                let device_code = self.state.input_buffer.trim().to_uppercase();
                if self.is_valid_device_code(&device_code) {
                    self.set_custom_device_code(device_code);
                } else {
                    self.state.status_message = "Invalid device code. Must be 8 alphanumeric characters.".to_string();
                }
            },
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                let device_code = self.state.input_buffer.trim().to_uppercase();
                if self.is_valid_device_code(&device_code) {
                    self.set_custom_device_code(device_code);
                } else {
                    self.state.status_message = "Invalid device code. Must be 8 alphanumeric characters.".to_string();
                }
            },
            KeyCode::Char(c) if c.is_alphanumeric() && self.state.input_buffer.len() < 8 => {
                self.state.input_buffer.push(c.to_ascii_uppercase());
            },
            KeyCode::Backspace => {
                self.state.input_buffer.pop();
            },
            _ => {}
        }
    }

    fn is_valid_device_code(&self, code: &str) -> bool {
        code.len() == 8 && code.chars().all(|c| c.is_ascii_alphanumeric())
    }

    fn set_custom_device_code(&mut self, code: String) {
        self.start_loading("Connecting with custom device...");

        let sender = self.async_sender.clone();
        self.runtime.spawn(async move {
            match async {
                let mut config = AppConfig::load()?;
                let mut client = BoardClient::new()?;
                let device_code = DeviceCode::from(code);
                client.set_device_code(device_code.clone());

                // Test the connection by trying to get pastes
                match client.get_all_pastes().await {
                    Ok(_) => {
                        // Save the device code to config
                        config.set_device_code(device_code.clone())?;
                        Ok((client, device_code, config))
                    }
                    Err(e) => Err(anyhow::Error::from(e)),
                }
            }.await {
                Ok((client, device_code, config)) => {
                    let _ = sender.send(AsyncResult::CustomDeviceSet(client, device_code, config));
                }
                Err(e) => {
                    let _ = sender.send(AsyncResult::Error(format!("Invalid device code: {}", e)));
                }
            }
        });
    }

    fn handle_view_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state.mode = AppMode::Main;
                self.state.view_scroll = 0;
            },
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.view_scroll > 0 {
                    self.state.view_scroll -= 1;
                }
            },
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.view_scroll += 1;
            },
            KeyCode::PageUp => {
                self.state.view_scroll = self.state.view_scroll.saturating_sub(10);
            },
            KeyCode::PageDown => {
                self.state.view_scroll += 10;
            },
            _ => {}
        }
    }

    fn handle_help_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.state.mode = AppMode::Main;
            },
            _ => {}
        }
    }

    fn handle_error_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                self.state.mode = AppMode::Main;
                self.state.error_message = None;
            },
            _ => {}
        }
    }

    fn start_loading(&mut self, message: &str) {
        self.state.loading_state = LoadingState::Loading(message.to_string());
        self.state.loading_start = Some(Instant::now());
    }

    fn stop_loading(&mut self) {
        self.state.loading_state = LoadingState::Idle;
        self.state.loading_start = None;
    }

    fn refresh_pastes(&mut self) {
        if let Some(client) = self.state.client.clone() {
            self.start_loading("Loading pastes...");

            let sender = self.async_sender.clone();
            self.runtime.spawn(async move {
                match client.get_all_pastes().await {
                    Ok(pastes) => {
                        let _ = sender.send(AsyncResult::PastesLoaded(pastes));
                    }
                    Err(e) => {
                        let _ = sender.send(AsyncResult::Error(format!("Failed to load pastes: {}", e)));
                    }
                }
            });
        }
    }

    fn register_new_device(&mut self) {
        self.start_loading("Registering new device...");

        let sender = self.async_sender.clone();
        self.runtime.spawn(async move {
            match async {
                let mut config = AppConfig::load()?;
                let mut client = BoardClient::new()?;
                let device_code = client.register_device().await?;

                // Save the device code to config
                config.set_device_code(device_code.clone())?;

                Ok::<(BoardClient, DeviceCode, AppConfig), anyhow::Error>((client, device_code, config))
            }.await {
                Ok((client, device_code, config)) => {
                    let _ = sender.send(AsyncResult::DeviceRegistered(client, device_code, config));
                }
                Err(e) => {
                    let _ = sender.send(AsyncResult::Error(format!("Failed to register device: {}", e)));
                }
            }
        });
    }

    fn create_paste(&mut self) {
        if self.state.input_buffer.trim().is_empty() {
            self.state.status_message = "Cannot create empty paste".to_string();
            return;
        }

        if let Some(client) = self.state.client.clone() {
            self.start_loading("Creating paste...");
            let content = self.state.input_buffer.clone();

            let sender = self.async_sender.clone();
            self.runtime.spawn(async move {
                match client.create_paste(&content).await {
                    Ok(paste) => {
                        let _ = sender.send(AsyncResult::PasteCreated(paste));
                    }
                    Err(e) => {
                        let _ = sender.send(AsyncResult::Error(format!("Failed to create paste: {}", e)));
                    }
                }
            });
        }
    }

    fn previous_paste(&mut self) {
        if self.state.pastes.is_empty() {
            return;
        }

        let selected = match self.state.selected_paste {
            Some(i) if i > 0 => i - 1,
            Some(_) => self.state.pastes.len() - 1,
            None => 0,
        };

        self.state.selected_paste = Some(selected);
        self.state.list_state.select(Some(selected));
    }

    fn next_paste(&mut self) {
        if self.state.pastes.is_empty() {
            return;
        }

        let selected = match self.state.selected_paste {
            Some(i) if i < self.state.pastes.len() - 1 => i + 1,
            Some(_) => 0,
            None => 0,
        };

        self.state.selected_paste = Some(selected);
        self.state.list_state.select(Some(selected));
    }

    fn view_selected_paste(&mut self) {
        if let Some(selected) = self.state.selected_paste {
            if selected < self.state.pastes.len() {
                self.state.mode = AppMode::ViewPaste;
                self.state.view_scroll = 0;
            }
        }
    }

    fn copy_paste_url(&mut self, url: String) {
        #[cfg(feature = "clipboard")]
        {
            // Try to copy to clipboard if the feature is enabled
            match std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(&url)?;
                Ok(())
            }) {
                Ok(Ok(_)) => {
                    self.state.status_message = format!("URL copied to clipboard: {}", url);
                    return;
                }
                _ => {
                    // Fall through to showing the URL
                }
            }
        }

        // Fallback: just show the URL
        self.state.status_message = format!("URL: {}", url);
    }

    fn update_scroll_state(&mut self) {
        self.state.scroll_state = ScrollbarState::new(self.state.pastes.len())
            .position(self.state.selected_paste.unwrap_or(0));
    }

    fn get_loading_indicator(&self) -> String {
        if let LoadingState::Loading(_) = self.state.loading_state {
            let frames = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
            let elapsed = self.state.loading_start
                .map(|start| start.elapsed().as_millis() / 100)
                .unwrap_or(0);
            frames[(elapsed % frames.len() as u128) as usize].to_string()
        } else {
            " ".to_string()
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        match self.state.mode {
            AppMode::Main => self.draw_main(f),
            AppMode::CreatePaste => self.draw_create_paste(f),
            AppMode::ViewPaste => self.draw_view_paste(f),
            AppMode::EnterDeviceCode => self.draw_enter_device_code(f),
            AppMode::Help => self.draw_help(f),
            AppMode::Error => self.draw_error(f),
        }
    }

    fn draw_main(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Main content
                Constraint::Length(5),  // Status bar (increased for help text)
            ])
            .split(f.area());

        // Header
        let device_info = if let Some(device_code) = &self.state.device_code {
            format!("Board TUI - Device: {}", device_code)
        } else {
            "Board TUI - No Device".to_string()
        };

        let header = Paragraph::new(device_info)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("Board API Client"));
        f.render_widget(header, chunks[0]);

        // Main content area
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),  // Paste list
                Constraint::Percentage(50),  // Paste preview
            ])
            .split(chunks[1]);

        // Paste list
        let pastes: Vec<ListItem> = self.state.pastes
            .iter()
            .enumerate()
            .map(|(i, paste)| {
                let content_preview = paste.content.lines().next()
                    .unwrap_or("")
                    .chars()
                    .take(40)
                    .collect::<String>();

                let preview = if paste.content.len() > 40 {
                    format!("{}...", content_preview)
                } else {
                    content_preview
                };

                let style = if Some(i) == self.state.selected_paste {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(format!("{}", paste.id), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled(preview, Style::default().fg(Color::Gray)),
                    ]),
                ]).style(style)
            })
            .collect();

        let paste_list = List::new(pastes)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Pastes ({})", self.state.pastes.len())))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(paste_list, main_chunks[0], &mut self.state.list_state);

        // Scrollbar for paste list
        if self.state.pastes.len() > 0 {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            f.render_stateful_widget(
                scrollbar,
                main_chunks[0].inner(Margin { horizontal: 0, vertical: 1 }),
                &mut self.state.scroll_state,
            );
        }

        // Paste preview
        let preview_content = if let Some(selected) = self.state.selected_paste {
            if selected < self.state.pastes.len() {
                let paste = &self.state.pastes[selected];
                let lines: Vec<Line> = paste.content.lines()
                    .take(20)
                    .map(|line| Line::from(line.to_string()))
                    .collect();
                Text::from(lines)
            } else {
                Text::from("No paste selected")
            }
        } else if self.state.pastes.is_empty() {
            Text::from(vec![
                Line::from("No pastes found."),
                Line::from(""),
                Line::from("Press 'c' to create a new paste"),
                Line::from("Press 'r' to refresh"),
            ])
        } else {
            Text::from("Select a paste to preview")
        };

        let preview = Paragraph::new(preview_content)
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Preview"));
        f.render_widget(preview, main_chunks[1]);

        // Status bar with loading indicator
        let status_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status message
                Constraint::Length(3),  // Help text (increased from 2 to 3)
            ])
            .split(chunks[2]);

        // Loading indicator and status message
        let loading_indicator = self.get_loading_indicator();
        let status_text = match &self.state.loading_state {
            LoadingState::Loading(msg) => format!("{} {}", loading_indicator, msg),
            LoadingState::Idle => self.state.status_message.clone(),
        };

        let status = Paragraph::new(status_text)
            .style(match self.state.loading_state {
                LoadingState::Loading(_) => Style::default().fg(Color::Yellow),
                LoadingState::Idle => Style::default().fg(Color::Green),
            })
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(status, status_chunks[0]);

        // Key bindings help
        let help_text = vec![
            Line::from("h=help c=create r=refresh n=new-device e=enter-code ↑↓/jk=navigate Enter=view Ctrl+D=url q=quit"),
        ];
        let help = Paragraph::new(Text::from(help_text))
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Controls"));
        f.render_widget(help, status_chunks[1]);

        // Show loading modal for device registration or other operations
        self.draw_loading_modal(f);
    }

    fn draw_create_paste(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),   // Header
                Constraint::Min(0),      // Input area
                Constraint::Length(3),   // Instructions
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Create New Paste")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Input area
        let input_text = if self.state.input_buffer.is_empty() {
            Text::from("Type your paste content here...")
        } else {
            Text::from(self.state.input_buffer.clone())
        };

        let input = Paragraph::new(input_text)
            .wrap(Wrap { trim: false })
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Content ({} chars)", self.state.input_buffer.len())));
        f.render_widget(input, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("Enter to save paste | Shift+Enter for new line | Esc to cancel")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Instructions"));
        f.render_widget(instructions, chunks[2]);

        // Show loading overlay if creating
        if let LoadingState::Loading(msg) = &self.state.loading_state {
            let loading_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Percentage(40),
                ])
                .split(f.area())[1];

            let loading_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(loading_area)[1];

            let loading_text = format!("{} {}", self.get_loading_indicator(), msg);
            let loading_widget = Paragraph::new(loading_text)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Loading"));

            f.render_widget(Clear, loading_area);
            f.render_widget(loading_widget, loading_area);
        }
    }

    fn draw_view_paste(&mut self, f: &mut Frame) {
        if let Some(selected) = self.state.selected_paste {
            if selected < self.state.pastes.len() {
                let paste = &self.state.pastes[selected];

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),   // Header
                        Constraint::Min(0),      // Content
                        Constraint::Length(3),   // Instructions
                    ])
                    .split(f.area());

                // Header
                let header = Paragraph::new(format!("Viewing Paste: {}", paste.id))
                    .style(Style::default().fg(Color::Cyan))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(header, chunks[0]);

                // Content
                let lines: Vec<Line> = paste.content.lines()
                    .skip(self.state.view_scroll)
                    .map(|line| Line::from(line.to_string()))
                    .collect();

                let content = Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Content | URL: {}", paste.url)));
                f.render_widget(content, chunks[1]);

                // Instructions
                let instructions = Paragraph::new("↑/↓ or j/k to scroll | Esc or q to go back")
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL).title("Instructions"));
                f.render_widget(instructions, chunks[2]);
            }
        }
    }

    fn draw_enter_device_code(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),   // Header
                Constraint::Length(5),   // Input area
                Constraint::Min(0),      // Spacer
                Constraint::Length(4),   // Instructions
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Enter Device Code")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Input area
        let input_display = format!("{:8}", self.state.input_buffer);
        let input_text = vec![
            Line::from(vec![
                Span::styled("Device Code: ", Style::default().fg(Color::Yellow)),
                Span::styled(&input_display, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("{}/8 characters", self.state.input_buffer.len()),
                    Style::default().fg(Color::Gray)
                ),
            ]),
        ];

        let input = Paragraph::new(Text::from(input_text))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Device Code Entry"));
        f.render_widget(input, chunks[1]);

        // Instructions
        let instructions_text = vec![
            Line::from("Enter an 8-character alphanumeric device code from another device."),
            Line::from(""),
            Line::from("Enter to submit | Esc to cancel"),
        ];

        let instructions = Paragraph::new(Text::from(instructions_text))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Instructions"));
        f.render_widget(instructions, chunks[3]);

        // Show loading overlay if connecting
        if let LoadingState::Loading(msg) = &self.state.loading_state {
            let loading_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(3),
                    Constraint::Percentage(40),
                ])
                .split(f.area())[1];

            let loading_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(loading_area)[1];

            let loading_text = format!("{} {}", self.get_loading_indicator(), msg);
            let loading_widget = Paragraph::new(loading_text)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Loading"));

            f.render_widget(Clear, loading_area);
            f.render_widget(loading_widget, loading_area);
        }
    }

    fn draw_help(&mut self, f: &mut Frame) {
        let help_text = vec![
            Line::from(vec![Span::styled("Board TUI Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("Main View:", Style::default().fg(Color::Yellow))]),
            Line::from("  h - Show this help"),
            Line::from("  c - Create new paste"),
            Line::from("  r - Refresh paste list"),
            Line::from("  n - Register new device"),
            Line::from("  e - Enter custom device code"),
            Line::from("  ↑/↓, j/k - Navigate pastes"),
            Line::from("  Enter - View selected paste"),
            Line::from("  Ctrl+D - Show paste URL"),
            Line::from("  q/Esc - Quit"),
            Line::from(""),
            Line::from(vec![Span::styled("Create Paste:", Style::default().fg(Color::Yellow))]),
            Line::from("  Type content normally"),
            Line::from("  Enter - Save paste"),
            Line::from("  Shift+Enter - New line"),
            Line::from("  Esc - Cancel"),
            Line::from(""),
            Line::from(vec![Span::styled("Enter Device Code:", Style::default().fg(Color::Yellow))]),
            Line::from("  Type alphanumeric characters"),
            Line::from("  Enter - Submit code"),
            Line::from("  Esc - Cancel"),
            Line::from(""),
            Line::from(vec![Span::styled("View Paste:", Style::default().fg(Color::Yellow))]),
            Line::from("  ↑/↓, j/k - Scroll content"),
            Line::from("  PageUp/PageDown - Fast scroll"),
            Line::from("  q/Esc - Go back"),
            Line::from(""),
            Line::from(vec![Span::styled("Loading:", Style::default().fg(Color::Yellow))]),
            Line::from("  Spinning indicator shows API operations"),
            Line::from("  Press q to quit during loading"),
        ];

        let help = Paragraph::new(Text::from(help_text))
            .wrap(Wrap { trim: true })
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help"));

        // Center the help dialog
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(f.area());

        let area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(area[1])[1];

        f.render_widget(Clear, area);
        f.render_widget(help, area);
    }

    fn draw_loading_modal(&mut self, f: &mut Frame) {
        // Show modal for ALL loading operations
        if let LoadingState::Loading(msg) = &self.state.loading_state {
            let loading_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(35),
                    Constraint::Length(5),
                    Constraint::Percentage(35),
                ])
                .split(f.area())[1];

            let loading_area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(loading_area)[1];

            let loading_text = vec![
                Line::from(vec![
                    Span::styled(self.get_loading_indicator(), Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::styled(msg.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Please wait...", Style::default().fg(Color::Gray)),
                ]),
            ];

            let loading_widget = Paragraph::new(Text::from(loading_text))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Loading")
                    .style(Style::default().fg(Color::Yellow)));

            f.render_widget(Clear, loading_area);
            f.render_widget(loading_widget, loading_area);
        }
    }

    fn draw_error(&mut self, f: &mut Frame) {
        if let Some(error) = &self.state.error_message {
            let error_text = vec![
                Line::from(vec![Span::styled("Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))]),
                Line::from(""),
                Line::from(error.clone()),
                Line::from(""),
                Line::from("Press any key to continue..."),
            ];

            let error_dialog = Paragraph::new(Text::from(error_text))
                .wrap(Wrap { trim: true })
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .style(Style::default().fg(Color::Red)));

            // Center the error dialog
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ])
                .split(f.area());

            let area = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Percentage(20),
                ])
                .split(area[1])[1];

            f.render_widget(Clear, area);
            f.render_widget(error_dialog, area);
        }
    }

    /// Get the current should_quit state for testing
    #[cfg(test)]
    pub fn should_quit(&self) -> bool {
        self.state.should_quit
    }

    /// Set the should_quit state for testing
    #[cfg(test)]
    pub fn set_should_quit(&mut self, should_quit: bool) {
        self.state.should_quit = should_quit;
    }
}