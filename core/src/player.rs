use std::path::Path;

use blake3::Hash;
use kira::backend::cpal;

use crate::{backend::PlaybackError, Backend, Library, Queue};

pub struct Player {
    backend: Backend,
    library: Library,
    queue: Queue,

    volume: f32,
}

impl Player {
    pub fn new() -> Result<Self, cpal::Error> {
        Ok(Player {
            backend: Backend::new()?,
            library: Library::new(),
            queue: Queue::new(),
            volume: 0.5,
        })
    }

    pub fn populate_library(&mut self, root: &Path) {
        self.library.populate(root);
    }

    pub fn play(&mut self) {
        self.backend.play();
    }

    pub fn pause(&mut self) {
        self.backend.pause();
    }

    pub fn seek(&mut self, pos: f64) {
        self.backend.seek(pos);
    }

    pub fn queue_track(&mut self, track_id: Hash) {
        self.queue.add(track_id);
    }

    pub fn shuffle_queue(&mut self) {
        self.queue.shuffle();
    }

    pub fn queue_library(&mut self) {
        if let Some(tracks) = &self.library.tracks {
            for entry in tracks.iter() {
                self.queue.add(*entry.key());
            }
        }
    }

    pub fn play_next(&mut self) -> Result<(), PlaybackError> {
        if let Some(track_id) = self.queue.next() {
            if let Some(tracks) = &self.library.tracks {
                if let Some(track) = tracks.get(&track_id) {
                    self.backend.load_and_play(&track.path, self.volume)?;
                }
            }
        }
        Ok(())
    }

    pub fn play_previous(&mut self) -> Result<(), PlaybackError> {
        if let Some(track_id) = self.queue.previous() {
            if let Some(tracks) = &self.library.tracks {
                if let Some(track) = tracks.get(&track_id) {
                    self.backend.load_and_play(&track.path, self.volume)?;
                }
            }
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), PlaybackError> {
        if self.backend.is_stopped() && self.queue.current().is_some() {
            self.play_next()?;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol;
        self.backend.set_volume(vol);
    }

    pub fn is_playing(&self) -> bool {
        self.backend.is_playing()
    }

    pub fn position(&self) -> f64 {
        self.backend.position()
    }

    pub fn current_track_id(&self) -> Option<Hash> {
        self.queue.current()
    }
}
