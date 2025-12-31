use std::path::Path;
use thiserror::Error;

use kira::backend::cpal;

use crate::{
    Backend, Colors, Config, ConfigError, Library, LibraryError, Queue, RgbaImage, Track, VisData,
    backend::PlaybackError,
};

pub struct Player {
    backend: Backend,
    library: Library,
    queue: Queue,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        Config::init()?;

        let player = Player {
            backend: Backend::new()?,
            library: Library::new()?,
            queue: Queue::new(),
        };

        Ok(player)
    }

    pub fn populate_library(&mut self, root: &Path) -> Result<(), LibraryError> {
        self.library.populate(root)
    }

    pub fn reload_library(&mut self) -> Result<(), LibraryError> {
        self.library.reload()
    }

    pub fn play(&mut self) -> Result<(), PlaybackError> {
        if self.backend.has_sound() {
            self.backend.play();
        } else {
            self.start_current()?
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        self.backend.pause();
    }

    pub fn seek(&mut self, pos: f64) {
        self.backend.seek(pos);
    }

    pub fn shuffle_queue(&mut self) {
        self.queue.shuffle();
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

        // Sort tracks by artist, then album, then title
        tracks.sort_by(|a, b| {
            a.track_artist_or_default()
                .cmp(&b.track_artist_or_default())
                .then_with(|| a.album_or_default().cmp(&b.album_or_default()))
                .then_with(|| a.title_or_default().cmp(&b.title_or_default()))
        });

        self.queue
            .add_many(tracks.into_iter().filter_map(|t| t.id()));
        Ok(())
    }

    fn play_track(&mut self, track_id: Option<i64>) -> Result<(), PlaybackError> {
        if let Some(track_id) = track_id {
            if let Ok(Some(track)) = self.library.query_track_from_id(track_id) {
                self.backend.load_and_play(track.path())?;
            }
        }
        Ok(())
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

    pub fn update(&mut self) -> Result<(), PlaybackError> {
        // is the 2nd check necessary
        if self.backend.is_stopped() && self.backend.has_sound() {
            self.start_next()?;
        }
        Ok(())
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

    pub fn clone_current_track(&self) -> Option<Track> {
        let track_id = self.queue.current_id()?;
        self.library.query_track_from_id(track_id).ok()?
    }

    pub fn artwork(&self, track: &Track, size: u32) -> Option<(RgbaImage, Colors)> {
        self.library.artwork(track, size)
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn vis_data(&self) -> VisData {
        self.backend.vis_data()
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
