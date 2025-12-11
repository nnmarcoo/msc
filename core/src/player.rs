use std::path::Path;
use std::sync::Arc;

use blake3::Hash;
use kira::backend::cpal;

use crate::{ArtCache, Backend, Library, LibraryError, Queue, Track, backend::PlaybackError};

pub struct Player {
    backend: Backend,
    library: Library,
    queue: Queue,
}

impl Player {
    pub fn new() -> Result<Self, cpal::Error> {
        Ok(Player {
            backend: Backend::new()?,
            library: Library::new(),
            queue: Queue::new(),
        })
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

    pub fn queue_track(&mut self, track_id: Hash) {
        self.queue.add(track_id);
    }

    pub fn queue_library(&mut self) {
        if let Some(tracks) = &self.library.tracks {
            for entry in tracks.iter() {
                self.queue.add(*entry.key());
            }
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

    pub fn clone_current_track(&self) -> Option<Track> {
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
}
