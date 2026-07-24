//! Application state and the update/view loop.
//!
//! Layout lives in [`crate::layout::Layout`] as data; [`render`] builds the
//! widget tree from it each frame. Pane messages carry the [`PaneId`] they
//! belong to, so duplicate panes of the same kind stay independent.

mod render;

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::every;
use iced::widget::container;
use iced::{Element, Event, Length, Subscription, Task, Theme, event, keyboard, window};
use verse_core::{Library, Player, Track};

use crate::config::Config;
use crate::layout::{Axis, DropZone, Layout, PaneId, SplitPath};
use crate::pane::{PaneKind, PaneMessage, PaneStates, view as pane_view};
use crate::styles;
use crate::tasks;

pub struct App {
    library: Library,
    player: Player,
    config: Config,

    layouts: Vec<Layout>,
    active_layout: usize,
    pane_states: PaneStates,
    edit_mode: bool,
    drag: Option<DividerDrag>,
    pane_drag: Option<PaneDrag>,
    window: iced::Size,

    scanning: bool,
    seeking: Option<f32>,
    status: Option<String>,
}

struct DividerDrag {
    path: SplitPath,
    axis: Axis,
    last: f32,
}

struct PaneDrag {
    source: PaneId,
    root_edge: Option<DropZone>,
    pane_zone: Option<(PaneId, DropZone)>,
}

impl PaneDrag {
    fn over(&self) -> Option<DropTarget> {
        if let Some(edge) = self.root_edge {
            return Some(DropTarget::RootEdge(edge));
        }
        self.pane_zone.map(|(id, zone)| DropTarget::Pane(id, zone))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropTarget {
    Pane(PaneId, DropZone),
    RootEdge(DropZone),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    ToggleLock(PaneId),
    DividerGrabbed(SplitPath),
    PaneGrabbed(PaneId),
    DropHovered(DropTarget),
    DropHoverEnded(DropTarget),
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
            layouts,
            active_layout,
            pane_states: PaneStates::default(),
            edit_mode: false,
            drag: None,
            pane_drag: None,
            window: iced::Size::new(1280.0, 720.0),
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

    fn after_layout_change(&mut self) {
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

    #[allow(clippy::needless_pass_by_value)]
    fn update_playback(&mut self, message: Message) {
        match message {
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
                let ids: Vec<i64> = self.library.available().filter_map(Track::id).collect();
                let _ = self.player.replace_queue(&self.library, ids);
            }
            _ => {}
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let _ = self.player.update(&self.library);
            }

            Message::PlayPause
            | Message::Next
            | Message::Previous
            | Message::Seek(_)
            | Message::SeekReleased
            | Message::Volume(_)
            | Message::CycleLoop
            | Message::Shuffle
            | Message::PlayTrack(_)
            | Message::EnqueueTrack(_)
            | Message::RemoveFromQueue(_)
            | Message::ClearQueue
            | Message::QueueAll => self.update_playback(message),

            Message::Pane(id, message) => self.pane_states.update(id, message),
            Message::SplitPane(id, axis) => {
                if self.layout_mut().split(id, axis, PaneKind::Empty).is_some() {
                    self.after_layout_change();
                }
            }
            Message::ClosePane(id) => {
                if self.layout_mut().close(id) {
                    self.pane_states.remove(id);
                    self.after_layout_change();
                }
            }
            Message::SetPaneKind(id, kind) => {
                self.layout_mut().set_kind(id, kind);
                self.pane_states.reset(id, kind);
                self.persist_layouts();
            }
            Message::ToggleLock(id) => {
                let locked = self.layout().is_locked(id);
                self.layout_mut()
                    .set_lock(id, if locked { None } else { Some(240.0) });
                self.persist_layouts();
            }
            Message::DividerGrabbed(path) => {
                if let Some(axis) = self.layout().split_axis(&path) {
                    self.drag = Some(DividerDrag {
                        path,
                        axis,
                        last: f32::NAN,
                    });
                }
            }
            Message::PaneGrabbed(_) | Message::DropHovered(_) | Message::DropHoverEnded(_) => {
                self.update_pane_drag(&message);
            }
            Message::ToggleEditMode => self.edit_mode = !self.edit_mode,
            Message::SelectLayout(index) => {
                if index < self.layouts.len() && index != self.active_layout {
                    self.active_layout = index;
                    self.after_layout_change();
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
        use iced::mouse;

        match event {
            Event::Window(window::Event::CloseRequested) => {
                self.persist_layouts();
                return iced::exit();
            }
            Event::Window(window::Event::Resized(size)) => {
                self.window = *size;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if self.drag.is_some() => {
                self.drag_divider_to(*position);
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.drag.is_some() =>
            {
                self.drag = None;
                self.persist_layouts();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.pane_drag.is_some() =>
            {
                self.drop_pane();
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                return self.handle_key(key, *modifiers);
            }
            _ => {}
        }
        Task::none()
    }

    fn update_pane_drag(&mut self, message: &Message) {
        match *message {
            Message::PaneGrabbed(id) => {
                self.pane_drag = Some(PaneDrag {
                    source: id,
                    root_edge: None,
                    pane_zone: None,
                });
            }
            Message::DropHovered(target) => {
                if let Some(drag) = &mut self.pane_drag {
                    match target {
                        DropTarget::RootEdge(edge) => drag.root_edge = Some(edge),
                        DropTarget::Pane(id, _) if id == drag.source => drag.pane_zone = None,
                        DropTarget::Pane(id, zone) => drag.pane_zone = Some((id, zone)),
                    }
                }
            }
            Message::DropHoverEnded(target) => {
                if let Some(drag) = &mut self.pane_drag {
                    match target {
                        DropTarget::RootEdge(edge) if drag.root_edge == Some(edge) => {
                            drag.root_edge = None;
                        }
                        DropTarget::Pane(id, _)
                            if drag.pane_zone.is_some_and(|(over, _)| over == id) =>
                        {
                            drag.pane_zone = None;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn drop_pane(&mut self) {
        if let Some(drag) = self.pane_drag.take()
            && let Some(target) = drag.over()
        {
            let source = drag.source;
            let moved = match target {
                DropTarget::Pane(pane, zone) => self.layout_mut().move_pane(source, pane, zone),
                DropTarget::RootEdge(edge) => {
                    self.layout_mut().move_pane_to_root_edge(source, edge)
                }
            };
            if moved {
                self.after_layout_change();
            }
        }
        self.pane_drag = None;
    }

    fn drag_divider_to(&mut self, cursor: iced::Point) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        let along = match drag.axis {
            Axis::Vertical => cursor.x,
            Axis::Horizontal => cursor.y,
        };

        if drag.last.is_nan() {
            drag.last = along;
            return;
        }

        let delta = along - drag.last;
        drag.last = along;

        let span = match drag.axis {
            Axis::Vertical => self.window.width,
            Axis::Horizontal => self.window.height,
        };
        let path = drag.path.clone();
        self.layout_mut().drag_divider(&path, delta, span);
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
        let edit_mode = self.edit_mode;
        let dragging = self.drag.as_ref().map(|drag| drag.axis);
        let pane_drag = self.pane_drag.as_ref();
        let over = pane_drag.and_then(PaneDrag::over);

        let panes = render::view(layout, edit_mode, dragging, &|id, kind, edit| {
            let drag = pane_view::DragContext {
                active: pane_drag.is_some(),
                drop_zone: match over {
                    Some(DropTarget::Pane(target, zone)) if target == id => Some(zone),
                    _ => None,
                },
            };
            pane_view::view(id, kind, layout.is_locked(id), edit, drag)
        });

        let body = if pane_drag.is_some() {
            let root_edge = match over {
                Some(DropTarget::RootEdge(edge)) => Some(edge),
                _ => None,
            };
            pane_view::root_edge_band(panes, root_edge)
        } else {
            panes
        };

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
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
