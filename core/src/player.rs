use std::path::Path;
use thiserror::Error;

use kira::backend::cpal;

use crate::{
    Album, Config, ConfigError, Library, LibraryError, Playlist, Queue, Track, VisData,
    backend::{Backend, BackendState, PlaybackError},
    queue::LoopMode,
};

pub struct Player {
    backend: Backend,
    library: Library,
    queue: Queue,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        Config::init()?;

        Ok(Player {
            backend: Backend::new()?,
            library: Library::new()?,
            queue: Queue::new(),
        })
    }

    pub fn populate_library(&mut self, root: &Path) -> Result<(), LibraryError> {
        self.library.populate(root)
    }

    pub fn reload_library(&mut self) -> Result<(), LibraryError> {
        self.library.reload()
    }

    pub fn query_all_tracks(&self) -> Result<Vec<Track>, LibraryError> {
        self.library.query_all_tracks()
    }

    pub fn query_n_tracks(&self, limit: i64) -> Result<Vec<Track>, LibraryError> {
        self.library.query_n_tracks(limit)
    }

    pub fn query_tracks_by_album(&self, album_name: &str) -> Result<Vec<Track>, LibraryError> {
        self.library.query_tracks_by_album(album_name)
    }

    pub fn query_tracks_by_artist(&self, artist_name: &str) -> Result<Vec<Track>, LibraryError> {
        self.library.query_tracks_by_artist(artist_name)
    }

    pub fn query_all_albums(&self) -> Result<Vec<Album>, LibraryError> {
        self.library.query_all_albums()
    }

    pub fn query_track_from_id(&self, id: i64) -> Result<Option<Track>, LibraryError> {
        self.library.query_track_from_id(id)
    }

    pub fn query_track_from_path(&self, path: &str) -> Result<Option<Track>, LibraryError> {
        self.library.query_track_from_path(path)
    }

    pub fn query_track_count(&self) -> Result<i64, LibraryError> {
        self.library.query_track_count()
    }

    pub fn create_playlist(&self, name: &str) -> Result<i64, LibraryError> {
        self.library.create_playlist(name)
    }

    pub fn get_all_playlists(&self) -> Result<Vec<Playlist>, LibraryError> {
        self.library.get_all_playlists()
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> Result<(), LibraryError> {
        self.library.rename_playlist(id, name)
    }

    pub fn delete_playlist(&self, id: i64) -> Result<(), LibraryError> {
        self.library.delete_playlist(id)
    }

    pub fn add_track_to_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<(), LibraryError> {
        self.library.add_track_to_playlist(playlist_id, track_id)
    }

    pub fn remove_track_from_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<(), LibraryError> {
        self.library
            .remove_track_from_playlist(playlist_id, track_id)
    }

    pub fn get_tracks_in_playlist(&self, playlist_id: i64) -> Result<Vec<Track>, LibraryError> {
        self.library.get_tracks_in_playlist(playlist_id)
    }

    pub fn set_playlist_cover(
        &self,
        playlist_id: i64,
        track_id: Option<i64>,
    ) -> Result<(), LibraryError> {
        self.library.set_playlist_cover(playlist_id, track_id)
    }

    pub fn clear_library(&mut self) -> Result<(), LibraryError> {
        self.queue.clear();
        self.backend.stop();
        self.library.clear_library()?;
        Config::clear_root()?;
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), PlaybackError> {
        match self.backend.state() {
            BackendState::Paused => self.backend.play(),
            BackendState::Idle | BackendState::Finished => self.start_current()?,
            BackendState::Playing => {}
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        self.backend.pause();
    }

    pub fn seek(&mut self, pos: f64) {
        self.backend.seek(pos);
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.backend.set_volume(vol);
    }

    pub fn is_playing(&self) -> bool {
        self.backend.is_playing()
    }

    pub fn position(&self) -> f64 {
        self.backend.position()
    }

    pub fn vis_data(&self) -> VisData {
        self.backend.vis_data()
    }

    pub fn update(&mut self) -> Result<(), PlaybackError> {
        if self.backend.state() == BackendState::Finished {
            self.start_next()?;
        }
        Ok(())
    }

    pub fn shuffle_queue(&mut self) {
        self.queue.shuffle();
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        self.queue.remove_index(index);
    }

    pub fn move_to_queue_front(&mut self, index: usize) {
        self.queue.move_front(index);
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.backend.stop();
    }

    pub fn queue_back(&mut self, track_id: i64) {
        self.queue.add(track_id);
    }

    pub fn queue_front(&mut self, track_id: i64) {
        self.queue.add_next(track_id);
    }

    pub fn queue_many(&mut self, track_ids: impl Iterator<Item = i64>) {
        self.queue.add_many(track_ids);
    }

    pub fn queue_library(&mut self) -> Result<(), LibraryError> {
        let mut tracks = self.library.query_all_tracks()?;

        tracks.sort_by(|a, b| {
            a.track_artist()
                .unwrap_or("-")
                .cmp(b.track_artist().unwrap_or("-"))
                .then_with(|| a.album().unwrap_or("-").cmp(b.album().unwrap_or("-")))
                .then_with(|| a.title().unwrap_or("-").cmp(b.title().unwrap_or("-")))
        });

        self.queue
            .add_many(tracks.into_iter().filter_map(|t| t.id()));
        Ok(())
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.queue.set_loop_mode(mode);
    }

    pub fn cycle_loop_mode(&mut self) -> LoopMode {
        self.queue.cycle_loop_mode()
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.queue.loop_mode()
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn start_next(&mut self) -> Result<(), PlaybackError> {
        let track_id = self.queue.next();
        self.play_track(track_id)
    }

    pub fn start_previous(&mut self) -> Result<(), PlaybackError> {
        let track_id = self.queue.previous();
        self.play_track(track_id)
    }

    pub fn start_current(&mut self) -> Result<(), PlaybackError> {
        let track_id = self.queue.current_id();
        self.play_track(track_id)
    }

    pub fn clone_current_track(&self) -> Option<Track> {
        let track_id = self.queue.current_id()?;
        self.library.query_track_from_id(track_id).ok()?
    }

    fn play_track(&mut self, track_id: Option<i64>) -> Result<(), PlaybackError> {
        if let Some(id) = track_id {
            if let Ok(Some(track)) = self.library.query_track_from_id(id) {
                self.backend.load_and_play(track.path())?;
            }
        }
        Ok(())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = Config::save_current();
    }
}

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("Backend error: {0}")]
    Backend(#[from] cpal::Error),
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("Library error: {0}")]
    Library(#[from] LibraryError),
}
