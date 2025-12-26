use std::{
    error::Error,
    fmt::{self, Display},
    path::Path,
    sync::Arc,
};

use blake3::Hash;
use kira::backend::cpal;

use crate::{
    ArtCache, Backend, Config, ConfigError, Library, LibraryError, Queue, Track, VisData,
    backend::PlaybackError,
};

pub struct Player {
    backend: Backend,
    library: Library,
    queue: Queue,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        let config = Config::load().unwrap_or_default();

        let mut library = Library::new();
        if let Some(root) = config.root {
            library.populate(&root);
        }

        let player = Player {
            backend: Backend::new()?,
            library,
            queue: Queue::new(),
        };

        Ok(player)
    }

    pub fn populate_library(&mut self, root: &Path) {
        self.library.populate(root);
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

    pub fn queue_back(&mut self, track_id: Hash) {
        self.queue.add(track_id);
    }

    pub fn queue_front(&mut self, track_id: Hash) {
        self.queue.add_next(track_id);
    }

    pub fn queue_many(&mut self, track_ids: impl Iterator<Item = Hash>) {
        self.queue.add_many(track_ids);
    }

    pub fn queue_library(&mut self) {
        if let Some(tracks) = &self.library.tracks {
            // all to sort queue, idk if this is nice
            let mut track_pairs: Vec<(Hash, Arc<Track>)> = tracks
                .iter()
                .map(|entry| (*entry.key(), Arc::clone(&entry.value())))
                .collect();

            track_pairs.sort_by(|a, b| {
                a.1.metadata
                    .artist_or_default()
                    .cmp(&b.1.metadata.artist_or_default())
                    .then_with(|| {
                        a.1.metadata
                            .album_or_default()
                            .cmp(&b.1.metadata.album_or_default())
                    })
                    .then_with(|| {
                        a.1.metadata
                            .title_or_default()
                            .cmp(&b.1.metadata.title_or_default())
                    })
            });

            self.queue
                .add_many(track_pairs.into_iter().map(|(hash, _)| hash));
        }
    }

    fn play_track(&mut self, track_id: Option<Hash>) -> Result<(), PlaybackError> {
        if let Some(track_id) = track_id {
            if let Some(tracks) = &self.library.tracks {
                if let Some(track) = tracks.get(&track_id) {
                    self.backend.load_and_play(&track.path)?;
                }
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

    pub fn clone_current_track(&self) -> Option<Arc<Track>> {
        self.library.track_from_id(self.queue.current_id()?)
    }

    pub fn art(&self) -> Arc<ArtCache> {
        Arc::clone(&self.library.art)
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
        let config = Config {
            root: self.library.root().cloned(),
        };
        let _ = config.save();
    }
}

#[derive(Debug)]
pub enum PlayerError {
    Backend(cpal::Error),
    Config(ConfigError),
}

impl Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerError::Backend(e) => write!(f, "Backend error: {}", e),
            PlayerError::Config(e) => write!(f, "Config error: {}", e),
        }
    }
}

impl Error for PlayerError {}

impl From<cpal::Error> for PlayerError {
    fn from(err: cpal::Error) -> Self {
        PlayerError::Backend(err)
    }
}

impl From<ConfigError> for PlayerError {
    fn from(err: ConfigError) -> Self {
        PlayerError::Config(err)
    }
}
