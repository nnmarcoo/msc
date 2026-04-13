use iced::keyboard::{self, Key, key};
use iced::time::every;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container, space};
use iced::{Element, Event, Length, Subscription, Task, Theme};
use verse_core::{Album, Player, Playlist, Track};
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use crate::art_cache::ArtCache;
use crate::components::preferences::PreferenceMessage;
use crate::components::{bottom_bar, preferences};
use crate::config::{Config, LayoutAxis, LayoutNode};
use crate::media_controls::MediaSession;
use crate::pane::{Pane, PaneType};
use crate::panes::collections::{CollectionsPane, ExpandedItem};
use crate::panes::{CollectionsMessage, ControlsMessage};
use crate::styles::set_radius;
use crate::window_handle;

pub struct App {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    edit_mode: bool,
    player: Player,
    volume: f32,
    previous_volume: f32,
    seeking_position: Option<f32>,
    layout_presets: Vec<pane_grid::Configuration<Pane>>,
    current_preset: usize,
    hovered_track: Option<i64>,
    hovered_card: Option<(bool, i64)>,
    media_session: Option<MediaSession>,
    cached_tracks: RefCell<Option<Vec<Track>>>,
    cached_albums: RefCell<Option<Vec<Album>>>,
    cached_playlists: RefCell<Option<Vec<Playlist>>>,
    art_cache: ArtCache,
    is_minimized: bool,
    config: Config,
    editing_config: Option<Config>,
    confirming_clear: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Split(pane_grid::Axis, pane_grid::Pane),
    Close(pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Controls(ControlsMessage),
    Collections(CollectionsMessage),
    LibraryPathSelected(Option<PathBuf>),
    SetLibrary,
    PaneTypeChanged(pane_grid::Pane, PaneType),
    BottomBar(bottom_bar::Message),
    ClearQueue,
    RemoveFromQueue(usize),
    MoveToQueueFront(usize),
    PlayTrack(i64),
    QueueLibrary,
    QueueBack(i64),
    QueueFront(i64),
    PlayAlbum(String, Option<String>),
    QueueAlbumNext(String, Option<String>),
    QueueAlbumBack(String, Option<String>),
    AddTrackToPlaylist(i64, i64),
    CardHovered(bool, i64),
    CardUnhovered,
    TrackHovered(i64),
    TrackUnhovered,
    OpenPreferences,
    Preference(preferences::PreferenceMessage),
    Event(Event),
}

impl Default for App {
    fn default() -> Self {
        let config = Config::load();
        set_radius(config.rounded);

        let default_layout = pane_grid::Configuration::Pane(Pane::new(PaneType::Library));

        let layout_presets: Vec<pane_grid::Configuration<Pane>> = if config.layouts.is_empty() {
            vec![default_layout]
        } else {
            config.layouts.iter().map(node_to_pane_config).collect()
        };

        let current_preset = config
            .current_layout
            .min(layout_presets.len().saturating_sub(1));

        let panes = pane_grid::State::with_configuration(layout_presets[current_preset].clone());
        let mut player = Player::new().expect("Failed to initialize player");
        player.set_volume(config.volume);

        Self {
            panes,
            focus: None,
            edit_mode: false,
            player,
            volume: config.volume,
            previous_volume: config.volume,
            seeking_position: None,
            layout_presets,
            current_preset,
            hovered_track: None,
            hovered_card: None,
            media_session: None,
            cached_tracks: RefCell::new(None),
            cached_albums: RefCell::new(None),
            cached_playlists: RefCell::new(None),
            art_cache: ArtCache::new(),
            is_minimized: false,
            config,
            editing_config: None,
            confirming_clear: false,
        }
    }
}

impl App {
    pub fn theme(&self) -> Theme {
        self.config.theme.clone()
    }

    fn invalidate_library_cache(&mut self) {
        *self.cached_tracks.borrow_mut() = None;
        *self.cached_albums.borrow_mut() = None;
        *self.cached_playlists.borrow_mut() = None;
        self.art_cache.invalidate();
        for (_, pane) in self.panes.iter_mut() {
            pane.invalidate_cache();
        }
    }

    fn invalidate_playlist_cache(&mut self) {
        *self.cached_playlists.borrow_mut() = None;
        for (_, pane) in self.panes.iter_mut() {
            if let Some(cp) = pane.content.as_any_mut().downcast_mut::<CollectionsPane>() {
                cp.playlist_art_keys.clear();
                cp.playlists_initialized = false;
                if matches!(cp.expanded, Some(ExpandedItem::Playlist(_))) {
                    cp.expanded = None;
                    cp.expanded_tracks.clear();
                    cp.expanded_cover = None;
                }
            }
        }
    }

    fn ensure_cached_playlists(&self) {
        let mut cache = self.cached_playlists.borrow_mut();
        if cache.is_none() {
            *cache = Some(self.player.get_all_playlists().unwrap_or_default());
        }
    }

    fn ensure_cached_tracks(&self) {
        let mut cache = self.cached_tracks.borrow_mut();
        if cache.is_none() {
            let mut tracks = self.player.query_all_tracks().unwrap_or_default();
            tracks.sort_by(|a, b| {
                a.track_artist()
                    .unwrap_or("-")
                    .cmp(b.track_artist().unwrap_or("-"))
                    .then_with(|| a.album().unwrap_or("-").cmp(b.album().unwrap_or("-")))
                    .then_with(|| a.title().unwrap_or("-").cmp(b.title().unwrap_or("-")))
            });
            *cache = Some(tracks);
        }
    }

    fn ensure_cached_albums(&self) {
        let mut cache = self.cached_albums.borrow_mut();
        if cache.is_none() {
            *cache = Some(self.player.query_all_albums().unwrap_or_default());
        }
    }

    fn save_current_layout(&mut self) {
        let node = self.panes.layout().clone();
        self.layout_presets[self.current_preset] = Self::layout_to_configuration(&self.panes, node);
    }

    fn persist_layouts(&mut self) {
        self.config.layouts = self
            .layout_presets
            .iter()
            .map(pane_config_to_node)
            .collect();
        self.config.current_layout = self.current_preset;
        self.config.save();
    }

    fn layout_to_configuration(
        panes: &pane_grid::State<Pane>,
        layout: pane_grid::Node,
    ) -> pane_grid::Configuration<Pane> {
        match layout {
            pane_grid::Node::Split {
                axis, ratio, a, b, ..
            } => pane_grid::Configuration::Split {
                axis,
                ratio,
                a: Box::new(Self::layout_to_configuration(panes, *a)),
                b: Box::new(Self::layout_to_configuration(panes, *b)),
            },
            pane_grid::Node::Pane(pane_id) => {
                pane_grid::Configuration::Pane(panes.get(pane_id).unwrap().clone())
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueueLibrary => {
                let _ = self.player.queue_library();
                let _ = self.player.play();
            }
            Message::Split(axis, pane) => {
                if let Some((new_pane, _)) =
                    self.panes.split(axis, pane, Pane::new(PaneType::Empty))
                {
                    self.focus = Some(new_pane);
                }
            }
            Message::Close(pane) => {
                if let Some((_, sibling)) = self.panes.close(pane) {
                    self.focus = Some(sibling);
                }
            }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
            }
            Message::Dragged(_) => {}
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
            }
            Message::Tick => {
                if self.media_session.is_none() {
                    let hwnd = window_handle::get_hwnd();
                    self.media_session = MediaSession::new(hwnd).ok();
                }

                let _ = self.player.update();

                self.art_cache.poll();

                for (_, pane) in self.panes.iter_mut() {
                    pane.update(&self.player, &mut self.art_cache);
                }

                if let Some(session) = &self.media_session {
                    for event in session.poll_events() {
                        match event {
                            souvlaki::MediaControlEvent::Play => {
                                let _ = self.player.play();
                            }
                            souvlaki::MediaControlEvent::Pause => {
                                self.player.pause();
                            }
                            souvlaki::MediaControlEvent::Toggle => {
                                if self.player.is_playing() {
                                    self.player.pause();
                                } else {
                                    let _ = self.player.play();
                                }
                            }
                            souvlaki::MediaControlEvent::Next => {
                                let _ = self.player.start_next();
                            }
                            souvlaki::MediaControlEvent::Previous => {
                                let _ = self.player.start_previous();
                            }
                            _ => {}
                        }
                    }
                }

                if !self.is_minimized {
                    if let Some(seeking_pos) = self.seeking_position {
                        if (self.player.position() as f32 - seeking_pos).abs() < 0.1 {
                            self.seeking_position = None;
                        }
                    }
                }

                if let Some(session) = &mut self.media_session {
                    if self.player.is_playing() {
                        session.set_playback(souvlaki::MediaPlayback::Playing { progress: None });
                    } else {
                        session.set_playback(souvlaki::MediaPlayback::Paused { progress: None });
                    }

                    if let Some(track) = self.player.clone_current_track() {
                        session.set_metadata(
                            track.title().unwrap_or("Unknown Title"),
                            track.track_artist().unwrap_or("Unknown Artist"),
                            track.album().unwrap_or("Unknown Album"),
                            Some(track.duration() as f64),
                        );
                    }
                }
            }
            Message::LibraryPathSelected(path) => {
                if let Some(path) = path {
                    let _ = self.player.populate_library(&path);
                    self.invalidate_library_cache();
                }
            }
            Message::SetLibrary => {
                if self.player.reload_library().is_err() {
                    return Task::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_title("Select Music Library Folder")
                                .pick_folder()
                                .await
                                .map(|handle| handle.path().to_path_buf())
                        },
                        Message::LibraryPathSelected,
                    );
                } else {
                    self.invalidate_library_cache();
                }
            }
            Message::Controls(msg) => match msg {
                ControlsMessage::PlayPause => {
                    if self.player.is_playing() {
                        self.player.pause();
                    } else {
                        let _ = self.player.play();
                    }
                }
                ControlsMessage::Previous => {
                    let _ = self.player.start_previous();
                }
                ControlsMessage::Next => {
                    let _ = self.player.start_next();
                }
                ControlsMessage::VolumeChanged(vol) => {
                    self.volume = vol;
                    self.player.set_volume(vol);
                    self.config.volume = vol;
                    self.config.save();
                }
                ControlsMessage::ToggleMute => {
                    if self.volume > 0.0 {
                        self.previous_volume = self.volume;
                        self.volume = 0.0;
                    } else {
                        self.volume = self.previous_volume;
                    }
                    self.player.set_volume(self.volume);
                    self.config.volume = self.volume;
                    self.config.save();
                }
                ControlsMessage::SeekChanged(pos) => {
                    self.seeking_position = Some(pos);
                }
                ControlsMessage::SeekReleased => {
                    if let Some(pos) = self.seeking_position {
                        self.player.seek(pos as f64);
                    }
                }
                ControlsMessage::ShuffleQueue => {
                    self.player.shuffle_queue();
                }
                ControlsMessage::CycleLoopMode => {
                    self.player.cycle_loop_mode();
                }
            },
            Message::PaneTypeChanged(pane_id, new_type) => {
                if let Some(pane) = self.panes.get_mut(pane_id) {
                    pane.set_content(new_type);
                }
            }
            Message::OpenPreferences => {
                self.editing_config = Some(self.config.clone());
            }
            Message::Preference(msg) => match msg {
                PreferenceMessage::SetTheme(t) => {
                    if let Some(c) = &mut self.editing_config {
                        c.theme = t;
                    }
                }
                PreferenceMessage::SetRounded(v) => {
                    if let Some(c) = &mut self.editing_config {
                        c.rounded = v;
                    }
                }
                PreferenceMessage::SetPresetIndicator(v) => {
                    if let Some(c) = &mut self.editing_config {
                        c.preset_indicator = v;
                    }
                }
                PreferenceMessage::Save => {
                    if let Some(mut c) = self.editing_config.take() {
                        c.layouts = self.config.layouts.clone();
                        c.current_layout = self.config.current_layout;
                        set_radius(c.rounded);
                        c.save();
                        self.config = c;
                        self.confirming_clear = false;
                    }
                }
                PreferenceMessage::Cancel => {
                    self.editing_config = None;
                    self.confirming_clear = false;
                    set_radius(self.config.rounded);
                }
                PreferenceMessage::Reset => {
                    let defaults = Config::default();
                    set_radius(defaults.rounded);
                    self.editing_config = Some(defaults);
                }
                PreferenceMessage::SetLibrary => {
                    return Task::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_title("Select Music Library Folder")
                                .pick_folder()
                                .await
                                .map(|handle| handle.path().to_path_buf())
                        },
                        Message::LibraryPathSelected,
                    );
                }
                PreferenceMessage::ClearLibrary => {
                    self.confirming_clear = true;
                }
                PreferenceMessage::CancelClearLibrary => {
                    self.confirming_clear = false;
                }
                PreferenceMessage::ConfirmClearLibrary => {
                    self.confirming_clear = false;
                    let _ = self.player.clear_library();
                    self.invalidate_library_cache();
                }
            },
            Message::BottomBar(msg) => {
                use bottom_bar::Message as BottomBarMessage;
                match msg {
                    BottomBarMessage::OpenPreferences => {
                        self.editing_config = Some(self.config.clone());
                    }
                    BottomBarMessage::ToggleEditMode => {
                        if self.edit_mode {
                            self.save_current_layout();
                            self.persist_layouts();
                        }
                        self.edit_mode = !self.edit_mode;
                    }
                    BottomBarMessage::SwitchPreset(index) => {
                        if index < self.layout_presets.len() && index != self.current_preset {
                            if self.edit_mode {
                                self.save_current_layout();
                            }
                            self.current_preset = index;
                            self.panes = pane_grid::State::with_configuration(
                                self.layout_presets[index].clone(),
                            );
                            self.focus = None;
                            self.persist_layouts();
                        }
                    }
                    BottomBarMessage::AddPreset => {
                        let new_preset = pane_grid::Configuration::Pane(Pane::new(PaneType::Empty));
                        self.layout_presets.push(new_preset.clone());
                        self.current_preset = self.layout_presets.len() - 1;
                        self.panes = pane_grid::State::with_configuration(new_preset);
                        self.focus = None;
                        self.persist_layouts();
                    }
                    BottomBarMessage::RemovePreset => {
                        if self.layout_presets.len() > 1 {
                            self.layout_presets.remove(self.current_preset);
                            self.current_preset =
                                self.current_preset.min(self.layout_presets.len() - 1);
                            self.panes = pane_grid::State::with_configuration(
                                self.layout_presets[self.current_preset].clone(),
                            );
                            self.focus = None;
                            self.persist_layouts();
                        }
                    }
                }
            }
            Message::ClearQueue => {
                self.player.clear_queue();
            }
            Message::RemoveFromQueue(index) => {
                self.player.remove_from_queue(index);
            }
            Message::MoveToQueueFront(index) => {
                self.player.move_to_queue_front(index);
            }
            Message::PlayTrack(track_id) => {
                if self
                    .player
                    .query_track_from_id(track_id)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    self.player.queue_front(track_id);
                    let _ = self.player.start_next();
                }
            }
            Message::QueueBack(track_id) => {
                if self
                    .player
                    .query_track_from_id(track_id)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    self.player.queue_back(track_id);
                }
            }
            Message::QueueFront(track_id) => {
                if self
                    .player
                    .query_track_from_id(track_id)
                    .ok()
                    .flatten()
                    .is_some()
                {
                    self.player.queue_front(track_id);
                }
            }
            Message::Collections(msg) => {
                for (_, pane) in self.panes.iter_mut() {
                    if let Some(cp) = pane.content.as_any_mut().downcast_mut::<CollectionsPane>() {
                        match &msg {
                            CollectionsMessage::ToggleNewPlaylistInput => {
                                cp.creating_playlist = !cp.creating_playlist;
                                cp.new_playlist_name.clear();
                            }
                            CollectionsMessage::NameChanged(name) => {
                                cp.new_playlist_name = name.clone();
                            }
                            CollectionsMessage::Confirm(_) => {
                                cp.creating_playlist = false;
                                cp.new_playlist_name.clear();
                            }
                            _ => {}
                        }
                    }
                }

                match msg {
                    CollectionsMessage::Confirm(name) => {
                        if !name.is_empty() {
                            let _ = self.player.create_playlist(&name);
                            self.invalidate_playlist_cache();
                        }
                    }
                    CollectionsMessage::DeletePlaylist(id) => {
                        let _ = self.player.delete_playlist(id);
                        self.invalidate_playlist_cache();
                    }
                    CollectionsMessage::PlayPlaylist(id) => {
                        if let Ok(tracks) = self.player.get_tracks_in_playlist(id) {
                            self.player.clear_queue();
                            self.player.queue_many(tracks.iter().filter_map(|t| t.id()));
                            let _ = self.player.play();
                        }
                    }
                    CollectionsMessage::QueuePlaylistNext(id) => {
                        if let Ok(tracks) = self.player.get_tracks_in_playlist(id) {
                            self.player
                                .queue_many_front(tracks.iter().filter_map(|t| t.id()));
                        }
                    }
                    CollectionsMessage::QueuePlaylistBack(id) => {
                        if let Ok(tracks) = self.player.get_tracks_in_playlist(id) {
                            self.player.queue_many(tracks.iter().filter_map(|t| t.id()));
                        }
                    }
                    CollectionsMessage::ToggleAlbum(name, artist) => {
                        let new_key = ExpandedItem::Album(name.clone(), artist.clone());
                        let fetched = self
                            .player
                            .query_tracks_by_album(&name, artist.as_deref())
                            .unwrap_or_default();
                        let album_id = {
                            let cache = self.cached_albums.borrow();
                            cache
                                .as_ref()
                                .and_then(|albums| albums.iter().find(|a| a.name == name))
                                .map(|a| a.id)
                        };
                        for (_, pane) in self.panes.iter_mut() {
                            if let Some(cp) =
                                pane.content.as_any_mut().downcast_mut::<CollectionsPane>()
                            {
                                if cp.expanded.as_ref() == Some(&new_key) {
                                    cp.expanded = None;
                                    cp.expanded_tracks.clear();
                                    cp.expanded_cover = None;
                                } else {
                                    cp.expanded = Some(new_key.clone());
                                    cp.expanded_tracks = fetched.clone();
                                    cp.expanded_cover = album_id
                                        .and_then(|aid| cp.album_art_keys.get(&aid))
                                        .map(|(tid, path)| (*tid, path.clone()));
                                }
                            }
                        }
                    }
                    CollectionsMessage::TogglePlaylist(id) => {
                        let new_key = ExpandedItem::Playlist(id);
                        let fetched = self.player.get_tracks_in_playlist(id).unwrap_or_default();
                        for (_, pane) in self.panes.iter_mut() {
                            if let Some(cp) =
                                pane.content.as_any_mut().downcast_mut::<CollectionsPane>()
                            {
                                if cp.expanded.as_ref() == Some(&new_key) {
                                    cp.expanded = None;
                                    cp.expanded_tracks.clear();
                                    cp.expanded_cover = None;
                                } else {
                                    cp.expanded = Some(new_key.clone());
                                    cp.expanded_tracks = fetched.clone();
                                    cp.expanded_cover = cp
                                        .playlist_art_keys
                                        .get(&id)
                                        .map(|(tid, path)| (*tid, path.clone()));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Message::AddTrackToPlaylist(track_id, playlist_id) => {
                let _ = self.player.add_track_to_playlist(playlist_id, track_id);
                self.invalidate_playlist_cache();
            }
            Message::PlayAlbum(album_name, artist) => {
                if let Ok(tracks) = self
                    .player
                    .query_tracks_by_album(&album_name, artist.as_deref())
                {
                    self.player.clear_queue();
                    self.player.queue_many(tracks.iter().filter_map(|t| t.id()));
                    let _ = self.player.play();
                }
            }
            Message::QueueAlbumNext(album_name, artist) => {
                if let Ok(tracks) = self
                    .player
                    .query_tracks_by_album(&album_name, artist.as_deref())
                {
                    self.player
                        .queue_many_front(tracks.iter().filter_map(|t| t.id()));
                }
            }
            Message::QueueAlbumBack(album_name, artist) => {
                if let Ok(tracks) = self
                    .player
                    .query_tracks_by_album(&album_name, artist.as_deref())
                {
                    self.player.queue_many(tracks.iter().filter_map(|t| t.id()));
                }
            }
            Message::CardHovered(is_album, id) => {
                self.hovered_card = Some((is_album, id));
            }
            Message::CardUnhovered => {
                self.hovered_card = None;
            }
            Message::TrackHovered(track_id) => {
                self.hovered_track = Some(track_id);
            }
            Message::TrackUnhovered => {
                self.hovered_track = None;
            }
            Message::Event(event) => match event {
                Event::Window(window_event) => match window_event {
                    iced::window::Event::Resized(size) => {
                        self.is_minimized = size.width == 0.0 && size.height == 0.0;
                    }
                    iced::window::Event::CloseRequested => {
                        std::process::exit(0);
                    }
                    _ => {}
                },
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key, modifiers: _, ..
                }) => match key {
                    Key::Named(key::Named::Space) => {
                        if self.player.is_playing() {
                            self.player.pause();
                        } else {
                            let _ = self.player.play();
                        }
                    }
                    Key::Character(c) => {
                        if let Ok(num) = c.parse::<usize>() {
                            if num >= 1 && num <= self.layout_presets.len() {
                                let index = num - 1;
                                if index == self.current_preset {
                                    return Task::none();
                                }
                                if self.edit_mode {
                                    self.save_current_layout();
                                }
                                self.current_preset = index;
                                self.panes = pane_grid::State::with_configuration(
                                    self.layout_presets[index].clone(),
                                );
                                self.focus = None;
                                self.persist_layouts();
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.is_minimized {
            return space().into();
        }

        let total_panes = self.panes.len();
        let edit_mode = self.edit_mode;

        self.ensure_cached_tracks();
        self.ensure_cached_albums();
        self.ensure_cached_playlists();

        let player = &self.player;
        let volume = self.volume;
        let hovered_track = &self.hovered_track;
        let hovered_card = self.hovered_card;
        let seeking_position = self.seeking_position;
        let cached_tracks = &self.cached_tracks;
        let cached_albums = &self.cached_albums;
        let cached_playlists = &self.cached_playlists;
        let art_cache = &self.art_cache;

        let mut pane_grid = PaneGrid::new(&self.panes, move |id, pane, _is_maximized| {
            pane.view(
                id,
                total_panes,
                edit_mode,
                player,
                volume,
                hovered_track,
                hovered_card,
                seeking_position,
                cached_tracks,
                cached_albums,
                cached_playlists,
                art_cache,
            )
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(if edit_mode { 6 } else { 0 });

        if edit_mode {
            pane_grid = pane_grid
                .on_click(Message::Clicked)
                .on_drag(Message::Dragged)
                .on_resize(10, Message::Resized);
        }

        let pane_grid_container = if edit_mode {
            container(pane_grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.background.strong.color.into()),
                        ..Default::default()
                    }
                })
        } else {
            container(pane_grid)
                .width(Length::Fill)
                .height(Length::Fill)
        };

        if let Some(pending) = &self.editing_config {
            return preferences::view(pending, &self.config.theme, self.confirming_clear)
                .map(Message::Preference);
        }

        column![
            pane_grid_container,
            bottom_bar::view(
                self.layout_presets.len(),
                self.current_preset,
                self.edit_mode,
                self.config.preset_indicator,
            )
            .map(Message::BottomBar)
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick_duration = if self.is_minimized {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(30)
        };

        Subscription::batch([
            every(tick_duration).map(|_| Message::Tick),
            iced::event::listen().map(Message::Event),
        ])
    }
}

fn pane_config_to_node(config: &pane_grid::Configuration<Pane>) -> LayoutNode {
    match config {
        pane_grid::Configuration::Pane(pane) => LayoutNode::Pane {
            pane_type: pane.pane_type.title().to_string(),
        },
        pane_grid::Configuration::Split { axis, ratio, a, b } => LayoutNode::Split {
            axis: match axis {
                pane_grid::Axis::Horizontal => LayoutAxis::Horizontal,
                pane_grid::Axis::Vertical => LayoutAxis::Vertical,
            },
            ratio: *ratio,
            a: Box::new(pane_config_to_node(a)),
            b: Box::new(pane_config_to_node(b)),
        },
    }
}

fn node_to_pane_config(node: &LayoutNode) -> pane_grid::Configuration<Pane> {
    match node {
        LayoutNode::Pane { pane_type } => {
            pane_grid::Configuration::Pane(Pane::new(PaneType::from_title(pane_type)))
        }
        LayoutNode::Split { axis, ratio, a, b } => pane_grid::Configuration::Split {
            axis: match axis {
                LayoutAxis::Horizontal => pane_grid::Axis::Horizontal,
                LayoutAxis::Vertical => pane_grid::Axis::Vertical,
            },
            ratio: *ratio,
            a: Box::new(node_to_pane_config(a)),
            b: Box::new(node_to_pane_config(b)),
        },
    }
}
