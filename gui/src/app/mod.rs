//! Application state and the update/view loop.
//!
//! Layout lives in [`crate::layout::Layout`] as data; the `pane_grid::State`
//! here is a derived view of it, rebuilt whenever the layout changes. Pane
//! messages carry the [`PaneId`] they belong to, so duplicate panes of the same
//! kind stay independent.

mod grid;

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::every;
use iced::widget::{PaneGrid, column, container, pane_grid};
use iced::{Element, Event, Length, Subscription, Task, Theme, event, keyboard, window};
use verse_core::{Library, Player};

use crate::components::transport;
use crate::config::Config;
use crate::layout::{Axis, Layout, PaneId};
use crate::pane::{PaneKind, PaneMessage, PaneStates, view as pane_view};
use crate::styles;
use crate::tasks;

pub struct App {
    library: Library,
    player: Player,
    config: Config,

    layouts: Vec<Layout>,
    active_layout: usize,
    panes: pane_grid::State<PaneId>,
    pane_states: PaneStates,
    edit_mode: bool,

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

    Pane(PaneId, PaneMessage),
    SplitPane(PaneId, Axis),
    ClosePane(PaneId),
    SetPaneKind(PaneId, PaneKind),
    PaneDragged(pane_grid::DragEvent),
    PaneResized(pane_grid::ResizeEvent),
    ToggleEditMode,
    SelectLayout(usize),

    SelectFolder,
    FolderPicked(PathBuf),
    Rescan,
    ScanFinished(Result<(), String>),

    Noop,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        styles::set_radius(config.rounded);

        let library = Library::open().expect("failed to open library");
        let mut player = Player::new().expect("failed to initialise audio");
        player.set_volume(config.volume);

        let mut layouts = config.layouts.clone();
        for layout in &mut layouts {
            layout.reconcile();
        }
        let active_layout = config.active_layout.min(layouts.len() - 1);

        let mut app = Self {
            library,
            player,
            config,
            panes: grid::build(&layouts[active_layout]),
            layouts,
            active_layout,
            pane_states: PaneStates::default(),
            edit_mode: false,
            scanning: false,
            seeking: None,
            status: None,
        };
        app.sync_pane_states();

        (app, Task::none())
    }

    fn layout(&self) -> &Layout {
        &self.layouts[self.active_layout]
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layouts[self.active_layout]
    }

    fn rebuild_grid(&mut self) {
        self.panes = grid::build(self.layout());
        self.sync_pane_states();
        self.persist_layouts();
    }

    fn sync_pane_states(&mut self) {
        let entries: Vec<_> = self
            .layout()
            .panes
            .iter()
            .map(|entry| (entry.id, entry.kind))
            .collect();

        for (id, kind) in &entries {
            self.pane_states.ensure(*id, *kind);
        }

        let live: Vec<PaneId> = entries.iter().map(|(id, _)| *id).collect();
        self.pane_states.retain(&live);
    }

    fn persist_layouts(&mut self) {
        self.config.layouts = self.layouts.clone();
        self.config.active_layout = self.active_layout;
        self.config.save();
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
            Message::Seek(position) => self.seeking = Some(position),
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
            Message::Shuffle => self.player.shuffle_queue(),

            Message::PlayTrack(id) => {
                let _ = self.player.play_now(&self.library, id);
            }
            Message::EnqueueTrack(id) => self.player.enqueue(id),
            Message::RemoveFromQueue(index) => self.player.remove_from_queue(index),
            Message::ClearQueue => self.player.clear_queue(),
            Message::QueueAll => {
                let ids: Vec<i64> = self.library.available().filter_map(|t| t.id()).collect();
                let _ = self.player.replace_queue(&self.library, ids);
            }

            Message::Pane(id, message) => self.pane_states.update(id, message),
            Message::SplitPane(id, axis) => {
                if self.layout_mut().split(id, axis, PaneKind::Empty).is_some() {
                    self.rebuild_grid();
                }
            }
            Message::ClosePane(id) => {
                if self.layout_mut().close(id) {
                    self.pane_states.remove(id);
                    self.rebuild_grid();
                }
            }
            Message::SetPaneKind(id, kind) => {
                self.layout_mut().set_kind(id, kind);
                self.pane_states.reset(id, kind);
                self.persist_layouts();
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
                grid::write_back(&self.panes, self.layout_mut());
                self.persist_layouts();
            }
            Message::PaneDragged(_) => {}
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                grid::write_back(&self.panes, self.layout_mut());
                self.persist_layouts();
            }
            Message::ToggleEditMode => self.edit_mode = !self.edit_mode,
            Message::SelectLayout(index) => {
                if index < self.layouts.len() && index != self.active_layout {
                    self.active_layout = index;
                    self.rebuild_grid();
                }
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
                        Err(error) => self.status = Some(format!("Reload failed: {error}")),
                    },
                    Err(error) => self.status = Some(format!("Scan failed: {error}")),
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
                self.persist_layouts();
                iced::exit()
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                self.handle_key(key, *modifiers)
            }
            _ => Task::none(),
        }
    }

    fn handle_key(&self, key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Task<Message> {
        use keyboard::key::Named;

        match key {
            keyboard::Key::Named(Named::Space) => Task::done(Message::PlayPause),
            keyboard::Key::Character(character) if modifiers.is_empty() => {
                if character.as_str() == "e" {
                    return Task::done(Message::ToggleEditMode);
                }
                match character.parse::<usize>() {
                    Ok(number) if (1..=self.layouts.len()).contains(&number) => {
                        Task::done(Message::SelectLayout(number - 1))
                    }
                    _ => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let layout = self.layout();

        let mut grid = PaneGrid::new(&self.panes, |_pane, id, _maximized| {
            let kind = layout.kind(*id).unwrap_or(PaneKind::Empty);
            pane_grid::Content::new(pane_view::view(
                *id,
                kind,
                &self.pane_states,
                &self.library,
                &self.player,
                self.edit_mode,
            ))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(if self.edit_mode { 6 } else { 0 });

        if self.edit_mode {
            grid = grid
                .on_drag(Message::PaneDragged)
                .on_resize(10, Message::PaneResized);
        }

        column![
            container(grid).height(Length::Fill),
            transport::view(transport::Context {
                library: &self.library,
                player: &self.player,
                seeking: self.seeking,
                scanning: self.scanning,
                status: self.status.as_deref(),
                edit_mode: self.edit_mode,
                layouts: &self.layouts,
                active_layout: self.active_layout,
            }),
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
