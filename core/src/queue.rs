use rand::seq::SliceRandom;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    #[default]
    None,
    /// Restart from the beginning once the last track finishes.
    Queue,
    /// Repeat the current track indefinitely.
    Single,
}

/// Playback order: what has played, what is playing, what is next.
///
/// Holds track ids rather than tracks, so reordering never touches metadata and
/// the queue stays valid across a rescan.
#[derive(Default)]
pub struct Queue {
    history: VecDeque<i64>,
    current: Option<i64>,
    upcoming: VecDeque<i64>,
    loop_mode: LoopMode,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<i64> {
        self.current
    }

    pub fn upcoming(&self) -> &VecDeque<i64> {
        &self.upcoming
    }

    pub fn history(&self) -> &VecDeque<i64> {
        &self.history
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.upcoming.is_empty()
    }

    pub fn len(&self) -> usize {
        self.upcoming.len() + usize::from(self.current.is_some())
    }

    /// Advances to the next track, or `None` when the queue is exhausted.
    pub fn advance(&mut self) -> Option<i64> {
        match self.loop_mode {
            LoopMode::Single => self.current,
            LoopMode::None => self.step(),
            LoopMode::Queue => self.step().or_else(|| {
                // Exhausted: everything played becomes the queue again, with
                // the track that just finished at the front — it was the first
                // thing enqueued this cycle.
                self.upcoming.extend(self.history.drain(..));
                if let Some(id) = self.current.take() {
                    self.upcoming.push_front(id);
                }
                self.step()
            }),
        }
    }

    /// Steps back through history, pushing the current track back onto the
    /// front of the queue. Stays put when there is nothing behind.
    pub fn go_back(&mut self) -> Option<i64> {
        if let Some(previous) = self.history.pop_back() {
            if let Some(current) = self.current.take() {
                self.upcoming.push_front(current);
            }
            self.current = Some(previous);
        }
        self.current
    }

    /// One position forward, ignoring loop mode.
    fn step(&mut self) -> Option<i64> {
        let next = self.upcoming.pop_front()?;
        if let Some(current) = self.current.take() {
            self.history.push_back(current);
        }
        self.current = Some(next);
        self.current
    }

    /// Enqueues at the end, or starts playing if nothing is current.
    pub fn push(&mut self, track_id: i64) {
        match self.current {
            None => self.current = Some(track_id),
            Some(_) => self.upcoming.push_back(track_id),
        }
    }

    /// Enqueues immediately after the current track.
    pub fn push_next(&mut self, track_id: i64) {
        match self.current {
            None => self.current = Some(track_id),
            Some(_) => self.upcoming.push_front(track_id),
        }
    }

    pub fn extend(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        let mut ids = track_ids.into_iter();
        if self.current.is_none() {
            self.current = ids.next();
        }
        self.upcoming.extend(ids);
    }

    /// Inserts a run of tracks directly after the current one, preserving their
    /// order relative to each other.
    pub fn extend_next(&mut self, track_ids: impl IntoIterator<Item = i64>) {
        let mut incoming: VecDeque<i64> = track_ids.into_iter().collect();
        if self.current.is_none() {
            self.current = incoming.pop_front();
        }
        incoming.extend(self.upcoming.drain(..));
        self.upcoming = incoming;
    }

    pub fn remove(&mut self, index: usize) -> Option<i64> {
        self.upcoming.remove(index)
    }

    pub fn move_to_front(&mut self, index: usize) {
        if let Some(track_id) = self.upcoming.remove(index) {
            self.upcoming.push_front(track_id);
        }
    }

    /// Shuffles what is still to come, leaving the current track playing.
    pub fn shuffle(&mut self) {
        let mut upcoming: Vec<i64> = self.upcoming.drain(..).collect();
        upcoming.shuffle(&mut rand::rng());
        self.upcoming = upcoming.into();
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.upcoming.clear();
        self.current = None;
    }

    /// Drops a track from every position, for when it leaves the library.
    pub fn remove_track(&mut self, track_id: i64) {
        self.history.retain(|&id| id != track_id);
        self.upcoming.retain(|&id| id != track_id);
        if self.current == Some(track_id) {
            self.current = None;
        }
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.loop_mode = mode;
    }

    pub fn cycle_loop_mode(&mut self) -> LoopMode {
        self.loop_mode = match self.loop_mode {
            LoopMode::None => LoopMode::Queue,
            LoopMode::Queue => LoopMode::Single,
            LoopMode::Single => LoopMode::None,
        };
        self.loop_mode
    }
}
