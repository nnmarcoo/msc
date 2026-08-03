//! Application state and the update/view loop.
//!
//! Layout lives in [`crate::layout::Layout`] as data; [`render`] builds the
//! widget tree from it each frame. Pane messages carry the [`PaneId`] they
//! belong to, so duplicate panes of the same kind stay independent.
//!
//! Track state (`search`, `selection`, `hovered`) is held here rather than per
//! pane, because it is keyed on track ids that every pane showing tracks reads;
//! see [`crate::browsing`]. [`RowClick`] names what a click's modifiers
//! meant so that raw key state is interpreted once, in the widget.
//!
//! `visible_ids` and `visible_albums` cache what the query matches, refreshed
//! together by `refresh_visible` wherever the query or the library changes and
//! nowhere else. Every pane listing tracks or covers reads them, and `update`
//! resolves row indices against the ids, so each filter runs once per change
//! rather than once per pane per frame, which on a large library cost more than
//! the frame budget by itself.
//!
//! The album cache matters for the same reason and is easy to miss: an album is
//! kept when any of its tracks matches, so filtering the grid walks the whole
//! library rather than the album list. Recomputing it in `view` therefore made a
//! pane of a few dozen covers cost a full library scan per frame, which is the
//! cost `visible_ids` already exists to avoid. Both caches are filled from one
//! [`crate::browsing::Context`], so they cannot disagree about the query
//! they were built from.
//!
//! Both hold owned keys rather than references because a `Vec<&Track>` or
//! `Vec<&Album>` on `App` would borrow from `App`, which Rust cannot express.
//! Panes resolve them against `library` when they draw, which is a map lookup
//! per visible row and a scan per drawn cover.
//!
//! A cached list is normally the thing to avoid here, since a row index only
//! means anything against the list the click was made on. What makes it safe is
//! that the query and the library are the filters' only two inputs: refreshing
//! on both leaves nothing that can change without the caches knowing. Adding a
//! third input to either filter means adding a `refresh_visible` call with it.
//!
//! `collection_track_ids` resolves a clicked cover through `visible_albums`, the
//! same cache the grid drew from, so the tile's number and the album it names
//! cannot disagree. Resolving against `library.albums()` instead was the bug this
//! replaced: that list is unfiltered, so under a query the number pointed at a
//! different record entirely. An index is only ever meaningful against one list,
//! and naming which one is what makes it safe rather than avoiding it — the same
//! reason [`crate::browsing::Selection`] pairs its anchor index with the id it
//! pointed at. A stale click resolves to nothing and plays nothing, since the
//! cache and the grid refresh together.
//!
//! Its playlist arm has no cache to resolve through, because there is nothing
//! expensive to cache: a playlist is filtered by its own name rather than by its
//! tracks, so the same filter runs here and in the pane. That it is one function,
//! [`crate::pane::collections::visible_playlists`], is what keeps the two in
//! step; two copies of the rule that drifted would leave a tile playing the
//! playlist beside the one pressed.
//!
//! Both arms drop tracks whose files are gone, so pressing play on a collection
//! queues only what can actually sound. The album arm once did not, which meant
//! a click on an album with a missing file queued silence at that position while
//! the same click on a playlist skipped it — one rule per kind for what is one
//! question. That the panel still *draws* missing tracks, dimmed, is a separate
//! matter: a list should read as the thing its owner made, but playing it should
//! not stall on a file the library has lost.
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
//! `NudgeVolume` is a step rather than a level because the keyboard has no
//! position to name, and it steps from `audible_volume` rather than from the
//! stored level so that the volume keys move the number the listener is hearing.
//! Nudging up from muted therefore lands one step above silence rather than
//! restoring the old level, which is the same thing the slider does when dragged
//! while muted: it is a level being named, and naming a level unmutes. The size
//! of a step is [`crate::widgets::volume::STEP`], shared with the slider's wheel
//! so that a key and a wheel click move the level equally.
//!
//! `config_dirty` exists because a volume drag names a new level on every
//! pointer move, and saving each one rewrote the whole config file per frame.
//! Changes mark it instead and a subscription flushes once a second while it is
//! set, so an idle session does no disk work; `CloseRequested` flushes too, so a
//! clean exit never loses the last second.
//!
//! `editing_config` is both the preferences state and the flag that the window
//! is in that mode, so `view` returns [`crate::preferences`] instead of the
//! panes whenever it is set. Holding the pending config rather than a bool means
//! there is no way to be in preferences without something to edit, and none of
//! the layout messages can reach a tree that is not on screen.
//!
//! Toggling the mode off discards, exactly as Cancel does, since the key is a
//! toggle rather than a commit and only Save says "keep this". Both paths
//! restore the corner radius from the live config, which is the one edit that
//! previews through a global instead of through the pending clone.
//!
//! `pane_options` is the pane whose settings modal is open, with the size that
//! pane was laid out at when it opened. The metrics are captured then because
//! only the pane knows its own size — it comes from the `responsive` closure
//! that draws the gear — and the modal covering it cannot ask again; locking
//! therefore freezes the shape the pane had when its settings were opened, which
//! is the shape still on screen behind the scrim. Cycling is ignored unless the
//! modal open is the one for that pane, so a message arriving after it closed
//! cannot resize a pane nobody is looking at.
//!
//! The modal must not outlive the pane it describes, so `after_layout_change` —
//! which every structural change already routes through — drops it if the pane
//! is gone, and leaving edit mode takes it too, since that is where it was
//! opened from. A modal also takes Escape before any binding resolves: it is
//! what the key means everywhere else, and letting a rebindable action fire from
//! behind the scrim would act on a pane the user cannot currently see.
//!
//! Keys reach [`crate::keybinds`] in every mode but one: while a keybind row is
//! capturing, the next key *is* the binding, so it goes to the row rather than
//! running whatever it is currently bound to. Capture is the gate rather than
//! the preferences view itself, because gating on the view would kill the
//! shortcut that closes it and leave the mouse as the only way out.
//!
//! With the view merely open, playback keys still work — a preferences window is
//! no reason to be unable to pause — while edit mode and layout switching do
//! not, since both address a pane tree that is not on screen. Layout switching
//! also writes the config, so letting it run mid-edit would put the live config
//! on disk in the middle of editing a copy of it.
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

static NO_SETTINGS: std::sync::LazyLock<PaneSettings> =
    std::sync::LazyLock::new(PaneSettings::default);

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::time::every;
use iced::widget::container;
use iced::{Element, Event, Length, Subscription, Task, Theme, event, keyboard, window};
use verse_core::{AlbumKey, Library, Player, Track};

use crate::artwork::{Cache as ArtCache, Decoded};
use crate::browsing::{Context, Selection};
use crate::config::Config;
use crate::keybinds::Action;
use crate::layout::{Axis, DropZone, Layout, Locks, PaneId, PaneMetrics, SplitPath};
use crate::pane::collections::{self, Kind as CollectionKind};
use crate::pane::settings::{PaneSettings, Settings as PaneSettingsValue};
use crate::pane::{PaneKind, PaneMessage, PaneStates, options as pane_options, view as pane_view};
use crate::preferences::{self, PreferenceMessage, PreferenceOutcome, PreferenceState};
use crate::styles;
use crate::tasks;
use crate::widgets::volume;

pub struct App {
    library: Library,
    player: Player,
    config: Config,
    artwork: ArtCache,

    layouts: Vec<Layout>,
    active_layout: usize,
    pane_states: PaneStates,
    edit_mode: bool,
    drag: Option<DividerDrag>,
    pane_drag: Option<PaneDrag>,
    pane_options: Option<(PaneId, PaneMetrics)>,

    editing_config: Option<Config>,
    preference_state: PreferenceState,

    search: String,
    selection: Selection,
    hovered: Option<i64>,

    visible_ids: Vec<i64>,
    visible_albums: Vec<AlbumKey>,

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
pub enum Act {
    Play,
    Next,
    Queue,
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
    NudgeVolume(f32),
    ToggleMute,
    CycleLoop,
    Shuffle,
    PlayTrack(i64),
    EnqueueTrack(i64),
    PlayTracks(Vec<i64>),
    Collection(CollectionKind, usize, Act),
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
    OpenPaneOptions(PaneId, PaneMetrics),
    ClosePaneOptions,
    CyclePaneLock(PaneId),
    SetPaneSettings(PaneId, PaneSettingsValue),
    DividerGrabbed(SplitPath, f32),
    PaneGrabbed(PaneId),
    DropHovered(DropTarget),
    DropHoverEnded(DropTarget),
    ToggleEditMode,
    SelectLayout(usize),

    TogglePreferences,
    Preference(PreferenceMessage),

    SelectFolder,
    FolderPicked(PathBuf),
    Rescan,
    ScanFinished(Result<(), String>),

    SaveConfig,

    ArtDecoded(Box<Decoded>),

    Noop,
}

impl App {
    pub fn new(config: Config) -> (Self, Task<Message>) {
        styles::set_radius(config.rounded);

        let library = Library::open().expect("failed to open library");
        let mut player = Player::new().expect("failed to initialize audio");
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
            artwork: ArtCache::new(),
            layouts,
            active_layout,
            pane_states: PaneStates::default(),
            edit_mode: false,
            drag: None,
            pane_drag: None,
            pane_options: None,
            editing_config: None,
            preference_state: PreferenceState::default(),
            search: String::new(),
            selection: Selection::default(),
            hovered: None,
            visible_ids: Vec::new(),
            visible_albums: Vec::new(),
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
        self.close_stale_pane_options();
        self.persist_layouts();
    }

    fn tick(&mut self) {
        let _ = self.player.update(&self.library);
        self.settle_seek();
    }

    fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
        if !self.edit_mode {
            self.pane_options = None;
        }
    }

    fn update_pane_options(&mut self, message: &Message) {
        match message {
            Message::OpenPaneOptions(id, size) => self.pane_options = Some((*id, *size)),
            Message::ClosePaneOptions => self.pane_options = None,
            Message::CyclePaneLock(id) => {
                if let Some((open, size)) = self.pane_options
                    && open == *id
                {
                    self.layout_mut().cycle_lock(*id, size);
                    self.persist_layouts();
                }
            }
            Message::SetPaneSettings(id, settings) => {
                self.layout_mut().set_settings(*id, *settings);
                self.persist_layouts();
            }
            _ => {}
        }
    }

    fn close_stale_pane_options(&mut self) {
        if let Some((id, _)) = self.pane_options
            && self.layout().kind(id).is_none()
        {
            self.pane_options = None;
        }
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

    fn toggle_preferences(&mut self) {
        self.editing_config = match self.editing_config {
            Some(_) => {
                styles::set_radius(self.config.rounded);
                None
            }
            None => Some(self.config.clone()),
        };
        self.preference_state = PreferenceState::default();
    }

    fn apply_preference(&mut self, message: PreferenceMessage) {
        let Some(pending) = self.editing_config.as_mut() else {
            return;
        };

        match preferences::update(message, pending, &mut self.preference_state) {
            PreferenceOutcome::Open => {}
            PreferenceOutcome::Save => {
                let pending = self.editing_config.take().expect("a pending config");
                self.config = pending;
                self.player.set_volume(self.audible_volume());
                self.config.save();
                self.config_dirty = false;
            }
            PreferenceOutcome::Cancel => {
                styles::set_radius(self.config.rounded);
                self.editing_config = None;
            }
        }
    }

    fn persist_layouts(&mut self) {
        self.config.layouts = self.layouts.clone();
        self.config.active_layout = self.active_layout;
        self.config.save();
        self.config_dirty = false;
    }

    fn refresh_visible(&mut self) {
        let context = self.context();
        let ids = context.matching_tracks().filter_map(Track::id).collect();
        let albums = context
            .matching_albums()
            .map(|album| album.key.clone())
            .collect();

        self.visible_ids = ids;
        self.visible_albums = albums;
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
                self.play_all(&ids);
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

    fn collection_track_ids(&self, kind: CollectionKind, shown: usize) -> Vec<i64> {
        match kind {
            CollectionKind::Album => self
                .visible_albums
                .get(shown)
                .and_then(|key| self.library.album(key))
                .map(|album| {
                    self.library
                        .album_tracks_available(album)
                        .filter_map(Track::id)
                        .collect()
                })
                .unwrap_or_default(),
            CollectionKind::Playlist => {
                let query = self.context().query();
                collections::visible_playlists(&self.library, &query)
                    .nth(shown)
                    .map(|playlist| {
                        playlist
                            .track_ids
                            .iter()
                            .filter_map(|&id| self.library.track(id))
                            .filter(|track| track.available())
                            .filter_map(Track::id)
                            .collect()
                    })
                    .unwrap_or_default()
            }
        }
    }

    fn selected_in_order(&self) -> Vec<i64> {
        self.selection.ordered_ids(self.visible_ids())
    }

    fn play_all(&mut self, ids: &[i64]) {
        if let Some((&first, rest)) = ids.split_first() {
            let _ = self.player.play_now(&self.library, first);
            self.player.queue_mut().extend_next(rest.iter().copied());
        }
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
            Message::NudgeVolume(step) => self.set_volume(self.audible_volume() + step),
            Message::ToggleMute => self.toggle_mute(),
            Message::CycleLoop => {
                self.player.cycle_loop_mode();
            }
            Message::Shuffle => self.player.shuffle_queue(),
            Message::PlayTrack(id) => {
                let _ = self.player.play_now(&self.library, id);
            }
            Message::EnqueueTrack(id) => self.player.enqueue(id),
            Message::PlayTracks(ids) => self.play_all(&ids),
            Message::Collection(kind, shown, act) => {
                let ids = self.collection_track_ids(kind, shown);
                match act {
                    Act::Play => self.play_all(&ids),
                    Act::Next => self.player.queue_mut().extend_next(ids),
                    Act::Queue => self.player.queue_mut().extend(ids),
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
        let task = self.dispatch(message);

        if self.artwork.is_idle() {
            return task;
        }

        Task::batch([task, self.decode_artwork()])
    }

    fn dispatch(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => self.tick(),

            Message::PlayPause
            | Message::Next
            | Message::Previous
            | Message::Seek(_)
            | Message::SeekReleased
            | Message::Volume(_)
            | Message::NudgeVolume(_)
            | Message::ToggleMute
            | Message::CycleLoop
            | Message::Shuffle
            | Message::PlayTrack(_)
            | Message::EnqueueTrack(_)
            | Message::PlayTracks(_)
            | Message::Collection(..)
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
            Message::OpenPaneOptions(..)
            | Message::ClosePaneOptions
            | Message::CyclePaneLock(_)
            | Message::SetPaneSettings(..) => {
                self.update_pane_options(&message);
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
            Message::ToggleEditMode => self.toggle_edit_mode(),
            Message::SelectLayout(index) => {
                if index < self.layouts.len() && index != self.active_layout {
                    self.active_layout = index;
                    self.after_layout_change();
                }
            }

            Message::TogglePreferences => self.toggle_preferences(),
            Message::Preference(message) => self.apply_preference(message),

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

            Message::ArtDecoded(decoded) => {
                let decoded = *decoded;
                if let Some((key, master)) = decoded.master {
                    self.artwork.keep_master(key, master);
                }
                self.artwork.insert(decoded.art);
            }

            Message::Event(event) => return self.handle_event(&event),
            Message::Noop => {}
        }
        Task::none()
    }

    fn decode_artwork(&mut self) -> Task<Message> {
        let jobs = self.artwork.take();
        if jobs.is_empty() {
            return Task::none();
        }

        Task::batch(
            jobs.into_iter()
                .map(|(job, master)| tasks::decode_art(job, master)),
        )
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
                    self.artwork.forget(&live);
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
            Event::Keyboard(keyboard::Event::KeyPressed {
                physical_key,
                modifiers,
                ..
            }) => {
                return self.handle_key(*physical_key, *modifiers);
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

    fn handle_key(
        &self,
        physical: keyboard::key::Physical,
        modifiers: keyboard::Modifiers,
    ) -> Task<Message> {
        if self.preference_state.capturing.is_some() {
            return match preferences::capture_key(&self.preference_state, physical, modifiers) {
                Some(message) => Task::done(Message::Preference(message)),
                None => Task::none(),
            };
        }

        if self.pane_options.is_some()
            && physical == keyboard::key::Physical::Code(keyboard::key::Code::Escape)
        {
            return Task::done(Message::ClosePaneOptions);
        }

        let open = self.editing_config.is_some();

        match self.config.keymap.resolve(physical, modifiers) {
            Some(Action::PlayPause) => Task::done(Message::PlayPause),
            Some(Action::Next) => Task::done(Message::Next),
            Some(Action::Previous) => Task::done(Message::Previous),
            Some(Action::ToggleMute) => Task::done(Message::ToggleMute),
            Some(Action::VolumeUp) => Task::done(Message::NudgeVolume(volume::STEP)),
            Some(Action::VolumeDown) => Task::done(Message::NudgeVolume(-volume::STEP)),
            Some(Action::CycleLoop) => Task::done(Message::CycleLoop),
            Some(Action::Shuffle) => Task::done(Message::Shuffle),
            Some(Action::TogglePreferences) => Task::done(Message::TogglePreferences),
            Some(Action::ToggleEditMode) => {
                if open {
                    Task::none()
                } else {
                    Task::done(Message::ToggleEditMode)
                }
            }
            Some(Action::SelectLayout(slot)) => {
                let index = usize::from(slot - 1);
                if open || index >= self.layouts.len() {
                    Task::none()
                } else {
                    Task::done(Message::SelectLayout(index))
                }
            }
            None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if let Some(pending) = &self.editing_config {
            return preferences::view(pending, &self.preference_state);
        }

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
                bins: *self.player.vis_data().bins(),
            },
            tracks: self.context(),
            visible,
            visible_albums: &self.visible_albums,
            artwork: &self.artwork,
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
                state: self.pane_states.get(id),
                settings: layout
                    .entry(id)
                    .map_or(&*NO_SETTINGS, |entry| &entry.settings),
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

        let root = container(body).width(Length::Fill).height(Length::Fill);

        match self.open_pane_options() {
            Some((id, kind, locks)) => iced::widget::stack![
                root,
                pane_options::view(id, kind, locks, &self.layout().settings(id))
            ]
            .into(),
            None => root.into(),
        }
    }

    fn open_pane_options(&self) -> Option<(PaneId, PaneKind, Locks)> {
        let (id, _) = self.pane_options?;
        let kind = self.layout().kind(id)?;
        Some((id, kind, self.layout().locks(id)))
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
        self.editing_config
            .as_ref()
            .unwrap_or(&self.config)
            .theme
            .clone()
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
