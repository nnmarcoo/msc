use thiserror::Error;

use kira::backend::cpal;

use crate::{
    Library, LoopMode, Queue, Track,
    analyzer::VisData,
    backend::{Backend, BackendState, PlaybackError},
};

/// Playback, and the order things play in.
///
/// Owns no library state. The methods that need to turn a track id into a file
/// take `&Library` explicitly, which keeps the two independent and makes the
/// dependency visible at every call site.
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

    /// Resumes if paused, otherwise starts the current track.
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

    /// Advances the queue when the current track has played out. Call this on a
    /// timer; it is a no-op unless the backend has finished.
    pub fn update(&mut self, library: &Library) -> Result<(), PlaybackError> {
        if self.backend.state() == BackendState::Finished {
            self.next(library)?;
        }
        Ok(())
    }

    pub fn next(&mut self, library: &Library) -> Result<(), PlaybackError> {
        match self.queue.advance() {
            Some(track_id) => self.start(library, track_id),
            None => {
                self.backend.stop();
                Ok(())
            }
        }
    }

    pub fn previous(&mut self, library: &Library) -> Result<(), PlaybackError> {
        // Restart the current track when it is already well underway, matching
        // what a "previous" button conventionally does.
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

    /// Plays a track, skipping ahead if it cannot be played.
    ///
    /// A missing file or an unreadable one advances the queue rather than
    /// stalling it, so one bad entry cannot wedge playback. The bound stops a
    /// queue full of missing tracks from recursing without end.
    fn start(&mut self, library: &Library, track_id: i64) -> Result<(), PlaybackError> {
        let mut track_id = track_id;

        for _ in 0..MAX_SKIPS {
            match library.track(track_id) {
                Some(track) if !track.missing() => {
                    match self.backend.load_and_play(track.path()) {
                        Ok(()) => return Ok(()),
                        // Tagged as present but unreadable now — treat it the
                        // same as missing and move on.
                        Err(PlaybackError::Load(_)) => {}
                        Err(err) => return Err(err),
                    }
                }
                _ => {}
            }

            match self.queue.advance() {
                Some(next) => track_id = next,
                None => break,
            }
        }

        self.backend.stop();
        Ok(())
    }
}

/// Enough to step over a stretch of missing files without letting a fully
/// unplayable queue spin.
const MAX_SKIPS: usize = 128;

/// Queue manipulation. These only reorder ids, so they need no library.
impl Player {
    pub fn play_now(&mut self, library: &Library, track_id: i64) -> Result<(), PlaybackError> {
        self.queue.push_next(track_id);
        self.next(library)
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

    /// Replaces the queue and starts playing.
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
