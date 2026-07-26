use thiserror::Error;

use kira::backend::cpal;

use crate::{
    Library, LoopMode, Queue, Track,
    analyzer::VisData,
    backend::{Backend, BackendState, PlaybackError},
};

pub struct Player {
    backend: Backend,
    queue: Queue,
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        Ok(Player {
            backend: Backend::new()?,
            queue: Queue::new(),
        })
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn queue_mut(&mut self) -> &mut Queue {
        &mut self.queue
    }

    pub fn current_track<'a>(&self, library: &'a Library) -> Option<&'a Track> {
        library.track(self.queue.current()?)
    }

    pub fn is_playing(&self) -> bool {
        self.backend.state() == BackendState::Playing
    }

    pub fn position(&self) -> f64 {
        self.backend.position()
    }

    pub fn volume(&self) -> f32 {
        self.backend.volume()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.backend.set_volume(volume);
    }

    pub fn seek(&mut self, position: f64) {
        self.backend.seek(position);
    }

    pub fn vis_data(&self) -> VisData {
        self.backend.vis_data()
    }

    pub fn pause(&mut self) {
        self.backend.pause();
    }

    pub fn play(&mut self, library: &Library) -> Result<(), PlaybackError> {
        match self.backend.state() {
            BackendState::Playing => Ok(()),
            BackendState::Paused => {
                self.backend.resume();
                Ok(())
            }
            BackendState::Idle | BackendState::Finished => self.start_current(library),
        }
    }

    pub fn toggle(&mut self, library: &Library) -> Result<(), PlaybackError> {
        if self.is_playing() {
            self.pause();
            Ok(())
        } else {
            self.play(library)
        }
    }

    pub fn stop(&mut self) {
        self.backend.stop();
    }

    pub fn update(&mut self, library: &Library) -> Result<(), PlaybackError> {
        if self.backend.state() == BackendState::Finished {
            self.next(library)?;
        }
        Ok(())
    }

    pub fn next(&mut self, library: &Library) -> Result<(), PlaybackError> {
        if let Some(track_id) = self.queue.advance() {
            self.start(library, track_id)
        } else {
            self.backend.stop();
            Ok(())
        }
    }

    pub fn previous(&mut self, library: &Library) -> Result<(), PlaybackError> {
        const RESTART_THRESHOLD: f64 = 3.0;
        if self.position() > RESTART_THRESHOLD {
            self.backend.seek(0.0);
            return Ok(());
        }

        match self.queue.go_back() {
            Some(track_id) => self.start(library, track_id),
            None => Ok(()),
        }
    }

    fn start_current(&mut self, library: &Library) -> Result<(), PlaybackError> {
        match self.queue.current() {
            Some(track_id) => self.start(library, track_id),
            None => self.next(library),
        }
    }

    fn start(&mut self, library: &Library, track_id: i64) -> Result<(), PlaybackError> {
        let mut track_id = track_id;

        loop {
            match library.track(track_id) {
                Some(track) if !track.missing() => match self.backend.load_and_play(track.path()) {
                    Ok(()) => return Ok(()),
                    Err(PlaybackError::Load(_)) => {}
                    Err(err) => return Err(err),
                },
                _ => {}
            }

            match self.queue.skip() {
                Some(next) => track_id = next,
                None => break,
            }
        }

        self.backend.stop();
        Ok(())
    }
}

impl Player {
    pub fn play_now(&mut self, library: &Library, track_id: i64) -> Result<(), PlaybackError> {
        self.queue.jump_to(track_id);
        self.start(library, track_id)
    }

    pub fn enqueue(&mut self, track_id: i64) {
        self.queue.push(track_id);
    }

    pub fn enqueue_next(&mut self, track_id: i64) {
        self.queue.push_next(track_id);
    }

    pub fn enqueue_all(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        self.queue.extend(track_ids);
    }

    pub fn enqueue_all_next(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        self.queue.extend_next(track_ids);
    }

    pub fn replace_queue(
        &mut self,
        library: &Library,
        track_ids: impl IntoIterator<Item = i64>,
    ) -> Result<(), PlaybackError> {
        self.queue.clear();
        self.queue.extend(track_ids);
        self.start_current(library)
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.backend.stop();
    }

    pub fn shuffle_queue(&mut self) {
        self.queue.shuffle();
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        self.queue.remove(index);
    }

    pub fn move_to_queue_front(&mut self, index: usize) {
        self.queue.move_to_front(index);
    }

    pub fn reorder_queue(&mut self, from: usize, to: usize) {
        self.queue.reorder(from, to);
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.queue.loop_mode()
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.queue.set_loop_mode(mode);
    }

    pub fn cycle_loop_mode(&mut self) -> LoopMode {
        self.queue.cycle_loop_mode()
    }
}

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("Audio backend error: {0}")]
    Backend(#[from] cpal::Error),
}
