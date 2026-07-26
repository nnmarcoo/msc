//! Application state and the update/view loop.
//!
//! Layout lives in [`crate::layout::Layout`] as data; [`render`] builds the
//! widget tree from it each frame. Pane messages carry the [`PaneId`] they
//! belong to, so duplicate panes of the same kind stay independent.
//!
//! Track state (`search`, `selection`, `hovered`) is held here rather than per
//! pane, because it is keyed on track ids that every pane showing tracks reads;
//! see [`crate::tracks`]. [`RowClick`] names what a click's modifiers meant so
//! that raw key state is interpreted once, in the widget.
//!
//! `visible_ids` caches the ids the query matches, refreshed by
//! `refresh_visible` wherever the query or the library changes and nowhere
//! else. Every pane listing tracks reads it, and `update` resolves row indices
//! against it, so the filter runs once per change rather than once per pane per
//! frame, which on a large library cost more than the frame budget by itself.
//!
//! The cache holds ids rather than `&Track` because a `Vec<&Track>` on `App`
//! would borrow from `App`. Panes resolve the ids against `library` when they
//! draw, which is a map lookup per visible row.
//!
//! A cached list is normally the thing to avoid here, since a row index only
//! means anything against the list the click was made on. What makes it safe is
//! that the query and the library are the filter's only two inputs: refreshing
//! on both leaves nothing that can change without the cache knowing. Adding a
//! third input to the filter means adding a `refresh_visible` call with it.
//!
//! A selection deliberately survives a query change: tracks filtered off screen
//! stay selected, so clearing a search restores what was picked before it. Only
//! a rescan calls `retain_listed`, where ids genuinely die. `RowRightClicked`
//! applies Explorer's rule, so that a right-click inside the selection keeps it
//! and one outside replaces it, and a menu never acts on rows out of sight.
//!
//! The selection is a set, so `selected_in_order` resolves it against the
//! visible rows before acting: queueing three rows plays them top to bottom.
//! Playing a set starts the first and queues the rest behind it, so "play" on a
//! multi-track selection means the whole selection.
//!
//! `seeking` holds the position a scrub is aiming at, and the timeline draws
//! from it in preference to the player. The audio moves once, on release: kira
//! restarts the stream at each `seek`, so seeking per pointer-move turns a drag
//! into a burst of quarter-second fragments. The rail still follows the pointer
//! throughout, so the scrub stays responsive without being audible.
//!
//! `seeking` then outlives the release. kira applies the seek on its own audio
//! thread, so `Player::position` keeps reporting the old spot for a frame or two
//! afterwards, and clearing `seeking` as the button came up made the rail snap
//! back and then jump forward again. `settle_seek` clears it only once the
//! player's own position agrees with the target to within `SEEK_SETTLED`, driven
//! by `Tick`. A single check at release would run before the audio thread had
//! caught up. The tick keeps running while a seek is outstanding, since a scrub
//! made while paused would otherwise have nothing to settle it.
//!
//! Volume lives in the config rather than being read back from the player, so
//! `audible_volume` is the one place the mute is applied: it returns zero while
//! muted, and that is both what the player is handed and what the volume panes
//! draw. Nothing else has to remember to check the flag, and a level and a mute
//! cannot disagree about what the listener hears. Naming a level unmutes, since
//! a drag is the user saying what they want to hear.
//!
//! `config_dirty` exists because a volume drag names a new level on every
//! pointer move, and saving each one rewrote the whole config file per frame.
//! Changes mark it instead and a subscription flushes once a second while it is
//! set, so an idle session does no disk work; `CloseRequested` flushes too, so a
//! clean exit never loses the last second.
//!
//! `Tick` drives animation, and `TICK` is 16ms because of it. Position comes
//! from `Player::position` when `view` runs, so a frame is only as fresh as the
//! last `view`, and anything following playback moves at this rate. At 250ms the
//! rail lurched.
//!
//! Widget-driven animation was tried instead and does not work here.
//! `Shell::request_redraw` repaints the existing widget tree; it does not re-run
//! `view`, so the timeline redrew at 60fps against a `position` captured
//! whenever `view` last ran. Smooth requests, stale values. A widget can animate
//! itself only from state it already owns, and the playhead is not that.
//!
//! The gate is a track being loaded rather than playback being active, which
//! costs an idle tick per frame while paused. The alternatives were worse:
//! `Player::is_playing` reads state kira's audio thread owns, so it still said
//! `false` right after a `PlayPause` and the window froze until a stray mouse
//! move repainted it, and a flag tracking what the user asked for cleared before
//! `pause`'s 500ms fade had been drawn.

mod render;

const SEEK_SETTLED: f32 = 0.35;

const TICK: Duration = Duration::from_millis(16);

const CONFIG_FLUSH: Duration = Duration::from_secs(1);

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::every;
use iced::widget::container;
use iced::{Element, Event, Length, Subscription, Task, Theme, event, keyboard, window};
use verse_core::{Library, Player, Track};

use crate::config::Config;
use crate::layout::{Axis, DropZone, Layout, PaneId, PaneMetrics, SplitPath};
use crate::pane::{PaneKind, PaneMessage, PaneStates, view as pane_view};
use crate::styles;
use crate::tasks;
use crate::tracks::{Context, Selection};

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

    search: String,
    selection: Selection,
    hovered: Option<i64>,

    visible_ids: Vec<i64>,

    scanning: bool,
    seeking: Option<f32>,
    config_dirty: bool,
    status: Option<String>,

    window: iced::Size,
}

struct DividerDrag {
    path: SplitPath,
    axis: Axis,
    last: f32,
    span: f32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowClick {
    Replace,
    Toggle,
    Extend,
}

impl RowClick {
    pub fn from_modifiers(modifiers: keyboard::Modifiers) -> Self {
        if modifiers.shift() {
            Self::Extend
        } else if modifiers.command() {
            Self::Toggle
        } else {
            Self::Replace
        }
    }
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
    ToggleMute,
    CycleLoop,
    Shuffle,
    PlayTrack(i64),
    EnqueueTrack(i64),
    PlayTracks(Vec<i64>),
    EnqueueTracks(Vec<i64>),
    EnqueueTracksNext(Vec<i64>),
    RemoveFromQueue(usize),
    ReorderQueue { from: usize, to: usize },
    ClearQueue,
    QueueAll,

    SearchChanged(String),
    TrackHovered(Option<i64>),
    RowClicked(usize, RowClick),
    RowActivated(usize),
    RowRightClicked(usize),
    SelectAll,
    ClearSelection,
    PlaySelection,
    QueueSelection,
    QueueSelectionNext,
    RateTrack(i64, Option<u8>),

    Pane(PaneId, PaneMessage),
    SplitPane(PaneId, Axis),
    ClosePane(PaneId),
    SetPaneKind(PaneId, PaneKind),
    CycleLock(PaneId, PaneMetrics),
    DividerGrabbed(SplitPath, f32),
    PaneGrabbed(PaneId),
    DropHovered(DropTarget),
    DropHoverEnded(DropTarget),
    ToggleEditMode,
    SelectLayout(usize),

    SelectFolder,
    FolderPicked(PathBuf),
    Rescan,
    ScanFinished(Result<(), String>),

    SaveConfig,

    Noop,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        styles::set_radius(config.rounded);

        let library = Library::open().expect("failed to open library");
        let mut player = Player::new().expect("failed to initialise audio");
        player.set_volume(if config.muted { 0.0 } else { config.volume });

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
            search: String::new(),
            selection: Selection::default(),
            hovered: None,
            visible_ids: Vec::new(),
            scanning: false,
            seeking: None,
            config_dirty: false,
            status: None,
            window: iced::Size::new(1280.0, 800.0),
        };
        app.sync_pane_states();
        app.refresh_visible();

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
        self.config_dirty = false;
    }

    fn visible(&self) -> Vec<&Track> {
        self.context().visible()
    }

    /// Refills the cached ids of the rows the query matches.
    ///
    /// The cache holds ids rather than `&Track` because a `Vec<&Track>` stored
    /// on `App` would borrow from `App`, which Rust cannot express. Ids are
    /// `Copy` and own nothing, so the panes resolve them against `library` in
    /// `view` and the filter itself runs once per change rather than once per
    /// pane per frame.
    ///
    /// This is the caching [`crate::tracks`] warns against, made safe by *when*
    /// it is refreshed: the query and the library are the filter's only inputs,
    /// so refilling wherever either changes leaves nothing that can go stale.
    /// The rule guards against a list that changed without the cache knowing,
    /// not against caching as such.
    fn refresh_visible(&mut self) {
        self.visible_ids = self
            .visible()
            .iter()
            .filter_map(|track| track.id())
            .collect();
    }

    fn set_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, verse_core::VOLUME_MAX);
        self.config.volume = volume;
        self.config.muted = false;
        self.player.set_volume(volume);
        self.config_dirty = true;
    }

    fn toggle_mute(&mut self) {
        self.config.muted = !self.config.muted;
        self.player.set_volume(self.audible_volume());
        self.config_dirty = true;
    }

    fn audible_volume(&self) -> f32 {
        if self.config.muted {
            0.0
        } else {
            self.config.volume
        }
    }

    fn flush_config(&mut self) {
        if self.config_dirty {
            self.config.save();
            self.config_dirty = false;
        }
    }

    fn settle_seek(&mut self) {
        let Some(target) = self.seeking else {
            return;
        };
        if (self.player.position() as f32 - target).abs() <= SEEK_SETTLED {
            self.seeking = None;
        }
    }

    fn context(&self) -> Context<'_> {
        Context {
            library: &self.library,
            queue: self.player.queue(),
            search: &self.search,
            selection: &self.selection,
            hovered: self.hovered,
            playing: self.player.queue().current(),
        }
    }

    fn visible_ids(&self) -> &[i64] {
        &self.visible_ids
    }

    fn update_tracks(&mut self, message: Message) {
        match message {
            Message::SearchChanged(query) => {
                self.search = query;
                self.refresh_visible();
            }
            Message::TrackHovered(id) => self.hovered = id,
            Message::RowClicked(index, click) => {
                let ids = std::mem::take(&mut self.visible_ids);
                if let Some(&id) = ids.get(index) {
                    match click {
                        RowClick::Replace => self.selection.select(index, id),
                        RowClick::Toggle => self.selection.toggle(index, id),
                        RowClick::Extend => self.selection.extend_to_ids(index, &ids),
                    }
                }
                self.visible_ids = ids;
            }
            Message::RowActivated(index) => {
                let ids = self.visible_ids();
                if let Some(&id) = ids.get(index) {
                    self.selection.select(index, id);
                    let _ = self.player.play_now(&self.library, id);
                }
            }
            Message::RowRightClicked(index) => {
                let ids = self.visible_ids();
                if let Some(&id) = ids.get(index)
                    && !self.selection.contains(id)
                {
                    self.selection.select(index, id);
                }
            }
            Message::SelectAll => {
                let ids = std::mem::take(&mut self.visible_ids);
                self.selection.select_all_ids(&ids);
                self.visible_ids = ids;
            }
            Message::ClearSelection => self.selection.clear(),
            Message::PlaySelection => {
                let ids = self.selected_in_order();
                if let Some((&first, rest)) = ids.split_first() {
                    let _ = self.player.play_now(&self.library, first);
                    self.player.queue_mut().extend_next(rest.iter().copied());
                }
            }
            Message::QueueSelection => {
                let ids = self.selected_in_order();
                self.player.queue_mut().extend(ids);
            }
            Message::QueueSelectionNext => {
                let ids = self.selected_in_order();
                self.player.queue_mut().extend_next(ids);
            }
            Message::RateTrack(id, stars) => {
                let _ = self.library.set_rating(id, stars);
                self.refresh_visible();
            }
            _ => {}
        }
    }

    fn selected_in_order(&self) -> Vec<i64> {
        self.selection.ordered_ids(self.visible_ids())
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
                if let Some(position) = self.seeking {
                    self.player.seek(f64::from(position));
                }
            }
            Message::Volume(volume) => self.set_volume(volume),
            Message::ToggleMute => self.toggle_mute(),
            Message::CycleLoop => {
                self.player.cycle_loop_mode();
            }
            Message::Shuffle => self.player.shuffle_queue(),
            Message::PlayTrack(id) => {
                let _ = self.player.play_now(&self.library, id);
            }
            Message::EnqueueTrack(id) => self.player.enqueue(id),
            Message::PlayTracks(ids) => {
                if let Some((&first, rest)) = ids.split_first() {
                    let _ = self.player.play_now(&self.library, first);
                    self.player.queue_mut().extend_next(rest.iter().copied());
                }
            }
            Message::EnqueueTracks(ids) => self.player.queue_mut().extend(ids),
            Message::EnqueueTracksNext(ids) => self.player.queue_mut().extend_next(ids),
            Message::RemoveFromQueue(index) => self.player.remove_from_queue(index),
            Message::ReorderQueue { from, to } => self.player.reorder_queue(from, to),
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
                self.settle_seek();
            }

            Message::PlayPause
            | Message::Next
            | Message::Previous
            | Message::Seek(_)
            | Message::SeekReleased
            | Message::Volume(_)
            | Message::ToggleMute
            | Message::CycleLoop
            | Message::Shuffle
            | Message::PlayTrack(_)
            | Message::EnqueueTrack(_)
            | Message::PlayTracks(_)
            | Message::EnqueueTracks(_)
            | Message::EnqueueTracksNext(_)
            | Message::RemoveFromQueue(_)
            | Message::ReorderQueue { .. }
            | Message::ClearQueue
            | Message::QueueAll => self.update_playback(message),

            Message::SearchChanged(_)
            | Message::TrackHovered(_)
            | Message::RowClicked(..)
            | Message::RowActivated(_)
            | Message::RowRightClicked(_)
            | Message::SelectAll
            | Message::ClearSelection
            | Message::PlaySelection
            | Message::QueueSelection
            | Message::QueueSelectionNext
            | Message::RateTrack(..) => self.update_tracks(message),

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
            Message::CycleLock(id, size) => {
                self.layout_mut().cycle_lock(id, size);
                self.persist_layouts();
            }
            Message::DividerGrabbed(path, span) => {
                if let Some(axis) = self.layout().split_axis(&path) {
                    self.drag = Some(DividerDrag {
                        path,
                        axis,
                        last: f32::NAN,
                        span,
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
            Message::ScanFinished(result) => self.finish_scan(&result),

            Message::SaveConfig => self.flush_config(),

            Message::Event(event) => return self.handle_event(&event),
            Message::Noop => {}
        }
        Task::none()
    }

    fn finish_scan(&mut self, result: &Result<(), String>) {
        self.scanning = false;

        let error = match result {
            Err(error) => Some(format!("Scan failed: {error}")),
            Ok(()) => match Library::open() {
                Err(error) => Some(format!("Reload failed: {error}")),
                Ok(library) => {
                    self.library = library;
                    self.refresh_visible();
                    let live: Vec<i64> =
                        self.library.tracks().iter().filter_map(Track::id).collect();
                    self.selection.retain_listed(&live);
                    self.hovered = self.hovered.filter(|id| live.contains(id));
                    self.status = Some(format!("{} tracks", self.library.tracks().len()));
                    None
                }
            },
        };

        if let Some(error) = error {
            self.status = Some(error);
        }
    }

    fn start_scan(&mut self, root: PathBuf) -> Task<Message> {
        if self.scanning {
            return Task::none();
        }
        self.scanning = true;
        self.status = Some("Scanning...".into());
        tasks::scan(root)
    }

    fn handle_event(&mut self, event: &Event) -> Task<Message> {
        use iced::mouse;

        match event {
            Event::Window(window::Event::Resized(size)) => self.window = *size,
            Event::Window(window::Event::CloseRequested) => {
                self.persist_layouts();
                self.flush_config();
                return iced::exit();
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

        let span = drag.span;
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
        let visible = &self.visible_ids;
        let edit_mode = self.edit_mode;
        let dragging = self.drag.as_ref().map(|drag| drag.axis);
        let pane_drag = self.pane_drag.as_ref();
        let over = pane_drag.and_then(PaneDrag::over);
        let shared = pane_view::Shared {
            playback: pane_view::Playback {
                is_playing: self.player.is_playing(),
                position: self
                    .seeking
                    .unwrap_or_else(|| self.player.position() as f32),
                volume: self.audible_volume(),
                muted: self.config.muted,
            },
            tracks: self.context(),
            visible,
        };

        let pane = move |id, kind, edit, span| {
            let drag = pane_view::DragContext {
                active: pane_drag.is_some(),
                drop_zone: match over {
                    Some(DropTarget::Pane(target, zone)) if target == id => Some(zone),
                    _ => None,
                },
            };
            let pane = pane_view::Pane {
                id,
                kind,
                locks: layout.locks(id),
                state: self.pane_states.get(id),
            };
            pane_view::view(pane, edit, drag, shared, span)
        };

        let panes = render::view(layout, edit_mode, dragging, &pane, self.window);

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
                "{} - {}",
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
        if self.player.queue().current().is_some() {
            subs.push(every(TICK).map(|_| Message::Tick));
        }
        if self.config_dirty {
            subs.push(every(CONFIG_FLUSH).map(|_| Message::SaveConfig));
        }

        Subscription::batch(subs)
    }
}
