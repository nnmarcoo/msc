use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use image::DynamicImage;
use msc_core::Player;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Resize, StatefulImage};
use std::{
    error::Error,
    io,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(PartialEq)]
enum View {
    Player,
    Queue,
    Help,
}

struct App {
    player: Player,
    view: View,
    volume: f32,
    pending_volume: Option<f32>,
    last_volume_change: Instant,
    library_path: String,
    last_update: Instant,
    should_quit: bool,
    message: Option<(String, Instant)>,
    image_picker: Picker,
    current_image: Option<Box<dyn StatefulProtocol>>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        let mut player = Player::new()?;
        player.set_volume(0.2);

        let font_size = (11, 24); // Reasonable default font size
        let mut image_picker = Picker::new(font_size);
        image_picker.guess_protocol();

        Ok(App {
            player,
            view: View::Player,
            volume: 0.2,
            pending_volume: None,
            last_volume_change: Instant::now(),
            library_path: String::from("D:\\audio"),
            last_update: Instant::now(),
            should_quit: false,
            message: None,
            image_picker,
            current_image: None,
        })
    }

    fn set_message(&mut self, msg: String) {
        self.message = Some((msg, Instant::now()));
    }

    fn get_message(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.message {
            if time.elapsed() < Duration::from_secs(3) {
                return Some(msg);
            }
        }
        None
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,

            KeyCode::Char('1') | KeyCode::F(1) => {
                self.view = View::Player;
            }
            KeyCode::Char('2') | KeyCode::F(2) => {
                self.view = View::Queue;
            }
            KeyCode::Char('?') | KeyCode::F(6) => {
                self.view = View::Help;
            }

            KeyCode::Char('l') => {
                self.player.populate_library(Path::new(&self.library_path));
                self.player.queue_library();
                self.set_message("Library loaded and queued".to_string());
            }

            KeyCode::Char('n') | KeyCode::Right => {
                if let Err(e) = self.player.start_next() {
                    self.set_message(format!("Error: {}", e));
                }
            }

            KeyCode::Char('p') | KeyCode::Left => {
                if let Err(e) = self.player.start_previous() {
                    self.set_message(format!("Error: {}", e));
                }
            }

            KeyCode::Char(' ') => {
                if self.player.is_playing() {
                    self.player.pause();
                    self.set_message("Paused".to_string());
                } else if let Err(e) = self.player.play() {
                    self.set_message(format!("Error: {}", e));
                }
            }

            KeyCode::Char('s') => {
                self.player.shuffle_queue();
                self.set_message("Queue shuffled".to_string());
            }

            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.volume = (self.volume + 0.05).min(1.0);
                self.pending_volume = Some(self.volume);
                self.last_volume_change = Instant::now();
                self.set_message(format!("Volume: {:.0}%", self.volume * 100.0));
            }

            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.volume = (self.volume - 0.05).max(0.0);
                self.pending_volume = Some(self.volume);
                self.last_volume_change = Instant::now();
                self.set_message(format!("Volume: {:.0}%", self.volume * 100.0));
            }

            _ => {}
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() >= Duration::from_millis(100) {
            if let Err(e) = self.player.update() {
                self.set_message(format!("Update error: {}", e));
            }
            self.last_update = Instant::now();
        }

        // Apply pending volume change after debounce period (150ms)
        if let Some(vol) = self.pending_volume {
            if self.last_volume_change.elapsed() >= Duration::from_millis(150) {
                self.player.set_volume(vol);
                self.pending_volume = None;
            }
        }

        // Update album art if track changed
        let should_update_art = if let Some(track) = self.player.current_track() {
            let art_cache = self.player.art();
            art_cache.get(&track)
        } else {
            None
        };

        if let Some(img) = should_update_art {
            if self.current_image.is_none() {
                self.update_album_art(img);
            }
        } else if self.current_image.is_some() {
            self.current_image = None;
        }
    }

    fn update_album_art(&mut self, img: Arc<DynamicImage>) {
        let dyn_img: &DynamicImage = &*img;
        let protocol = self.image_picker.new_resize_protocol(dyn_img.clone());
        self.current_image = Some(protocol);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

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
    match app.view {
        View::Player => render_player_view(f, app),
        View::Queue => render_queue_view(f, app),
        View::Help => render_help_view(f, app),
    }
}

fn render_player_view(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Content (art + track info)
            Constraint::Length(3),  // Progress bar
            Constraint::Length(3),  // Volume
            Constraint::Length(1),  // Status/message
        ])
        .split(f.area());

    render_header(f, main_chunks[0], app);

    // Split content area into album art and track info
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Album art
            Constraint::Percentage(60),  // Track info
        ])
        .split(main_chunks[1]);

    render_album_art(f, content_chunks[0], app);
    render_track_info(f, content_chunks[1], app);

    render_progress(f, main_chunks[2], app);
    render_volume(f, main_chunks[3], app);
    render_status_bar(f, main_chunks[4], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.player.is_playing() {
        "♫ MSC Player"
    } else {
        "⏸ MSC Player"
    };

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn render_album_art(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Album Art");

    if let Some(ref protocol) = app.current_image {
        let image = StatefulImage::new(None).resize(Resize::Fit(None));
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_stateful_widget(image, inner, &mut protocol.clone());
    } else {
        let no_art = Paragraph::new("No album art")
            .style(Style::default().fg(Color::DarkGray).italic())
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(no_art, area);
    }
}

fn render_track_info(f: &mut Frame, area: Rect, app: &App) {
    let track_lines = if let Some(track) = app.player.current_track() {
        let title = track.metadata.title_or_default();
        let artist = track.metadata.artist_or_default();
        let album = track.metadata.album_or_default();
        let genre = track.metadata.genre_or_default();
        let duration = track.metadata.duration();

        vec![
            Line::from(""),
            Line::from(
                Span::styled(
                    title.to_string(),
                    Style::default().fg(Color::White).bold().add_modifier(Modifier::UNDERLINED),
                )
            ).alignment(Alignment::Center),
            Line::from(""),
            Line::from(
                Span::styled(
                    artist.to_string(),
                    Style::default().fg(Color::LightYellow),
                )
            ).alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("Album: ", Style::default().fg(Color::DarkGray)),
                Span::styled(album.to_string(), Style::default().fg(Color::Gray)),
            ]).alignment(Alignment::Center),
            Line::from(""),
            Line::from(vec![
                Span::styled("Genre: ", Style::default().fg(Color::DarkGray)),
                Span::styled(genre.to_string(), Style::default().fg(Color::Gray)),
                Span::raw("  "),
                Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format_duration(duration as f64), Style::default().fg(Color::Gray)),
            ]).alignment(Alignment::Center),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(""),
            Line::from(
                Span::styled(
                    "No track loaded",
                    Style::default().fg(Color::DarkGray).italic(),
                )
            ).alignment(Alignment::Center),
            Line::from(""),
            Line::from(
                Span::styled(
                    "Press 'l' to load library",
                    Style::default().fg(Color::DarkGray),
                )
            ).alignment(Alignment::Center),
        ]
    };

    let track_info = Paragraph::new(track_lines)
        .block(Block::default().borders(Borders::ALL).title("Now Playing"))
        .wrap(Wrap { trim: true });

    f.render_widget(track_info, area);
}

fn render_progress(f: &mut Frame, area: Rect, app: &App) {
    let (position, duration) = if let Some(track) = app.player.current_track() {
        (app.player.position(), track.metadata.duration())
    } else {
        (0.0, 0.0)
    };

    let progress_ratio = if duration > 0.0 {
        (position / duration as f64).min(1.0)
    } else {
        0.0
    };

    let label = format!("{} / {}", format_duration(position), format_duration(duration as f64));

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .ratio(progress_ratio)
        .label(label);

    f.render_widget(progress, area);
}

fn render_volume(f: &mut Frame, area: Rect, app: &App) {
    let volume_percent = (app.volume * 100.0) as u16;
    let volume_bars = (app.volume * 20.0) as usize;
    let bars = "▮".repeat(volume_bars) + &"▯".repeat(20 - volume_bars);

    let volume = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Volume"))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent(volume_percent)
        .label(format!("{} {}%", bars, volume_percent));

    f.render_widget(volume, area);
}

fn render_queue_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(5),     // Queue list
            Constraint::Length(1),  // Status
        ])
        .split(f.area());

    render_header(f, chunks[0], app);

    let queue = app.player.queue();
    let mut queue_items = Vec::new();

    // Add current track
    if let Some(current_id) = queue.current() {
        let track_text = if let Some(track) = app.player.library().track_from_id(current_id) {
            let title = track.metadata.title_or_default();
            let artist = track.metadata.artist_or_default();
            format!("▶ {} - {}", title, artist)
        } else {
            "▶ Unknown Track".to_string()
        };
        queue_items.push(
            ListItem::new(track_text)
                .style(Style::default().fg(Color::Cyan).bold())
        );
    }

    // Add upcoming tracks
    for track_id in queue.upcoming().iter() {
        let track_text = if let Some(track) = app.player.library().track_from_id(*track_id) {
            let title = track.metadata.title_or_default();
            let artist = track.metadata.artist_or_default();
            format!("  {} - {}", title, artist)
        } else {
            "  Unknown Track".to_string()
        };
        queue_items.push(
            ListItem::new(track_text)
                .style(Style::default().fg(Color::White))
        );
    }

    let queue_list = List::new(queue_items)
        .block(Block::default().borders(Borders::ALL).title("Queue"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(queue_list, chunks[1]);
    render_status_bar(f, chunks[2], app);
}

fn render_help_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(5),     // Help content
            Constraint::Length(1),  // Status
        ])
        .split(f.area());

    render_header(f, chunks[0], app);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("Views", Style::default().fg(Color::Cyan).bold())),
        Line::from("  1 / F1     Player view"),
        Line::from("  2 / F2     Queue view"),
        Line::from("  ? / F6     Help (this screen)"),
        Line::from(""),
        Line::from(Span::styled("Playback", Style::default().fg(Color::Cyan).bold())),
        Line::from("  Space      Play / Pause"),
        Line::from("  n / →      Next track"),
        Line::from("  p / ←      Previous track"),
        Line::from(""),
        Line::from(Span::styled("Library", Style::default().fg(Color::Cyan).bold())),
        Line::from("  l          Load library"),
        Line::from("  s          Shuffle queue"),
        Line::from(""),
        Line::from(Span::styled("Audio", Style::default().fg(Color::Cyan).bold())),
        Line::from("  + / =      Volume up"),
        Line::from("  - / _      Volume down"),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(Color::Cyan).bold())),
        Line::from("  q          Quit"),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Key Bindings"))
        .alignment(Alignment::Left);

    f.render_widget(help, chunks[1]);
    render_status_bar(f, chunks[2], app);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_text = if let Some(msg) = app.get_message() {
        msg.to_string()
    } else {
        match app.view {
            View::Player => "1:Player  2:Queue  ?:Help  q:Quit".to_string(),
            View::Queue => "1:Player  2:Queue  ?:Help  q:Quit".to_string(),
            View::Help => "Press any view key to return".to_string(),
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(status, area);
}

fn format_duration(seconds: f64) -> String {
    let mins = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{:02}:{:02}", mins, secs)
}
