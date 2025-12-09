use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use msc_core::Player;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use std::{
    error::Error,
    io,
    path::Path,
    time::{Duration, Instant},
};

struct App {
    player: Player,
    status: String,
    volume: f32,
    library_path: String,
    last_update: Instant,
    should_quit: bool,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let player = Player::new()?;
        Ok(App {
            player,
            status: String::from("Ready - Press 'h' for help"),
            volume: 0.2,
            library_path: String::from("D:\\audio"),
            last_update: Instant::now(),
            should_quit: false,
        })
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('l') => {
                self.player.populate_library(Path::new(&self.library_path));
                self.player.queue_library();
                self.status = String::from("Library loaded and queued");
            }
            KeyCode::Char('n') | KeyCode::Right => {
                if let Err(e) = self.player.play_next() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = String::from("Playing next track");
                }
            }
            KeyCode::Char('p') | KeyCode::Left => {
                if let Err(e) = self.player.play_previous() {
                    self.status = format!("Error: {}", e);
                } else {
                    self.status = String::from("Playing previous track");
                }
            }
            KeyCode::Char(' ') => {
                if self.player.is_playing() {
                    self.player.pause();
                    self.status = String::from("Paused");
                } else {
                    self.player.play();
                    self.status = String::from("Resumed");
                }
            }
            KeyCode::Char('s') => {
                self.player.shuffle_queue();
                self.status = String::from("Queue shuffled");
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.volume = (self.volume + 0.05).min(1.0);
                self.player.set_volume(self.volume);
                self.status = format!("Volume: {:.0}%", self.volume * 100.0);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.volume = (self.volume - 0.05).max(0.0);
                self.player.set_volume(self.volume);
                self.status = format!("Volume: {:.0}%", self.volume * 100.0);
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        // Update player state every 100ms
        if self.last_update.elapsed() >= Duration::from_millis(100) {
            if let Err(e) = self.player.update() {
                self.status = format!("Update error: {}", e);
            }
            self.last_update = Instant::now();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Poll for events with timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        app.update();

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Status
            Constraint::Length(5), // Track info
            Constraint::Length(3), // Position
            Constraint::Length(3), // Volume
            Constraint::Min(5),    // Help
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("MSC Music Player - TUI")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Status
    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[1]);

    // TRACK INFO
    let track_text: Vec<Line> = if let Some(track) = app.player.current_track() {
        vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Yellow)),
                Span::raw(track.metadata.title_or_default().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(Color::Yellow)),
                Span::raw(track.metadata.artist_or_default().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Album:  ", Style::default().fg(Color::Yellow)),
                Span::raw(track.metadata.album_or_default().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Genre:  ", Style::default().fg(Color::Yellow)),
                Span::raw(track.metadata.genre_or_default().to_string()),
            ]),
            Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:.2}s", track.metadata.duration())),
            ]),
        ]
    } else {
        vec![Line::from(Span::raw("No track loaded".to_string()))]
    };

    let track_info = Paragraph::new(track_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Track"),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(track_info, chunks[2]);

    // Position
    let position_text = if app.player.is_playing() {
        format!("Position: {:.2}s", app.player.position())
    } else {
        String::from("Not playing")
    };
    let position = Paragraph::new(position_text)
        .style(Style::default().fg(Color::Magenta))
        .block(Block::default().borders(Borders::ALL).title("Playback"));
    f.render_widget(position, chunks[3]);

    // Volume gauge
    let volume_percent = (app.volume * 100.0) as u16;
    let volume = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Volume"))
        .gauge_style(Style::default().fg(Color::Blue))
        .percent(volume_percent);
    f.render_widget(volume, chunks[4]);

    // Help
    let help_items = vec![
        ListItem::new("Space: Play/Pause"),
        ListItem::new("n/→: Next track"),
        ListItem::new("p/←: Previous track"),
        ListItem::new("l: Load library"),
        ListItem::new("s: Shuffle queue"),
        ListItem::new("+/-: Volume up/down"),
        ListItem::new("q: Quit"),
    ];
    let help = List::new(help_items)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .style(Style::default().fg(Color::White));
    f.render_widget(help, chunks[5]);
}
