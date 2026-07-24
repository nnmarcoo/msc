use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::every;
use iced::widget::{column, container, row};
use iced::{Element, Event, Length, Subscription, Task, Theme, event, keyboard, window};
use verse_core::{Library, Player};

use crate::components::{library_list, queue_list, transport};
use crate::config::Config;
use crate::styles;
use crate::tasks;

pub struct App {
    library: Library,
    player: Player,
    config: Config,

    scanning: bool,
    seeking: Option<f32>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Event(Event),

    PlayPause,
    Next,
    Previous,
    Seek(f32),
    SeekReleased,
    Volume(f32),
    CycleLoop,
    Shuffle,

    PlayTrack(i64),
    EnqueueTrack(i64),
    RemoveFromQueue(usize),
    ClearQueue,
    QueueAll,

    SelectFolder,
    FolderPicked(PathBuf),
    Rescan,
    ScanFinished(Result<(), String>),

    Noop,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        styles::set_radius(config.rounded);

        let library = Library::open().unwrap_or_else(|e| {
            panic!("failed to open library: {e}");
        });

        let mut player = Player::new().expect("failed to initialise audio");
        player.set_volume(config.volume);

        let app = Self {
            library,
            player,
            config,
            scanning: false,
            seeking: None,
            status: None,
        };
        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let _ = self.player.update(&self.library);
            }

            Message::PlayPause => {
                let _ = self.player.toggle(&self.library);
            }
            Message::Next => {
                let _ = self.player.next(&self.library);
            }
            Message::Previous => {
                let _ = self.player.previous(&self.library);
            }
            Message::Seek(position) => {
                self.seeking = Some(position);
            }
            Message::SeekReleased => {
                if let Some(position) = self.seeking.take() {
                    self.player.seek(f64::from(position));
                }
            }
            Message::Volume(volume) => {
                self.player.set_volume(volume);
                self.config.volume = volume;
                self.config.save();
            }
            Message::CycleLoop => {
                self.player.cycle_loop_mode();
            }
            Message::Shuffle => {
                self.player.shuffle_queue();
            }

            Message::PlayTrack(id) => {
                let _ = self.player.play_now(&self.library, id);
            }
            Message::EnqueueTrack(id) => {
                self.player.enqueue(id);
            }
            Message::RemoveFromQueue(index) => {
                self.player.remove_from_queue(index);
            }
            Message::ClearQueue => {
                self.player.clear_queue();
            }
            Message::QueueAll => {
                let ids: Vec<i64> = self
                    .library
                    .available()
                    .filter_map(verse_core::Track::id)
                    .collect();
                let _ = self.player.replace_queue(&self.library, ids);
            }

            Message::SelectFolder => return tasks::pick_library_folder(),
            Message::FolderPicked(path) => return self.start_scan(path),
            Message::Rescan => {
                if let Some(root) = self.library.root().map(Path::to_path_buf) {
                    return self.start_scan(root);
                }
                self.status = Some("No library folder set".into());
            }
            Message::ScanFinished(result) => {
                self.scanning = false;
                match result {
                    Ok(()) => match Library::open() {
                        Ok(library) => {
                            self.library = library;
                            self.status = Some(format!("{} tracks", self.library.tracks().len()));
                        }
                        Err(e) => self.status = Some(format!("Reload failed: {e}")),
                    },
                    Err(e) => self.status = Some(format!("Scan failed: {e}")),
                }
            }

            Message::Event(event) => return self.handle_event(&event),
            Message::Noop => {}
        }
        Task::none()
    }

    fn start_scan(&mut self, root: PathBuf) -> Task<Message> {
        if self.scanning {
            return Task::none();
        }
        self.scanning = true;
        self.status = Some("Scanning…".into());
        tasks::scan(root)
    }

    fn handle_event(&mut self, event: &Event) -> Task<Message> {
        match event {
            Event::Window(window::Event::CloseRequested) => {
                self.config.save();
                iced::exit()
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) => Task::done(Message::PlayPause),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let body = row![
            container(library_list::view(&self.library, &self.player))
                .width(Length::FillPortion(3))
                .height(Length::Fill),
            container(queue_list::view(&self.library, &self.player))
                .width(Length::FillPortion(2))
                .height(Length::Fill)
                .style(styles::panel_style),
        ];

        column![
            container(body).height(Length::Fill),
            transport::view(
                &self.library,
                &self.player,
                self.seeking,
                self.scanning,
                self.status.as_deref(),
            ),
        ]
        .into()
    }

    pub fn title(&self) -> String {
        match self.player.current_track(&self.library) {
            Some(track) => format!(
                "{} — {}",
                track.title().unwrap_or("Unknown"),
                track.track_artist().unwrap_or("Unknown Artist")
            ),
            None => "Verse".into(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.config.theme.clone()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let events = event::listen_with(|event, status, _window| match status {
            event::Status::Ignored => Some(Message::Event(event)),
            event::Status::Captured => None,
        });

        let mut subs = vec![events];
        if self.player.is_playing() {
            subs.push(every(Duration::from_millis(100)).map(|_| Message::Tick));
        }

        Subscription::batch(subs)
    }
}
