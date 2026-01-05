use rand::seq::SliceRandom;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    None,
    Queue,
    Single,
}

impl Default for LoopMode {
    fn default() -> Self {
        LoopMode::None
    }
}

pub struct Queue {
    history: VecDeque<i64>,
    current: Option<i64>,
    upcoming: VecDeque<i64>,
    loop_mode: LoopMode,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            history: VecDeque::new(),
            current: None,
            upcoming: VecDeque::new(),
            loop_mode: LoopMode::None,
        }
    }

    pub fn add_next(&mut self, track_id: i64) {
        self.upcoming.push_front(track_id);
    }

    pub fn add(&mut self, track_id: i64) {
        if self.current.is_none() {
            self.current = Some(track_id);
        } else {
            self.upcoming.push_back(track_id);
        }
    }

    pub fn add_many(&mut self, track_ids: impl Iterator<Item = i64>) {
        if self.current.is_none() {
            let mut ids = track_ids;
            self.current = ids.next();
            self.upcoming.extend(ids);
        } else {
            self.upcoming.extend(track_ids);
        }
    }

    pub fn next(&mut self) -> Option<i64> {
        match self.loop_mode {
            LoopMode::Single => {
                // Just return current track without advancing
                self.current
            }
            LoopMode::Queue => {
                if let Some(next) = self.upcoming.pop_front() {
                    // Normal advance within queue
                    if let Some(current) = self.current.take() {
                        self.history.push_back(current);
                    }
                    self.current = Some(next);
                } else if self.current.is_some() && !self.history.is_empty() {
                    // Reached end of queue - restart from beginning
                    // Move current to history
                    if let Some(current) = self.current.take() {
                        self.history.push_back(current);
                    }
                    // Move all history back to upcoming (in order)
                    self.upcoming.extend(self.history.drain(..));
                    // Pop the first track as current
                    self.current = self.upcoming.pop_front();
                }
                self.current
            }
            LoopMode::None => {
                if let Some(next) = self.upcoming.pop_front() {
                    if let Some(current) = self.current.take() {
                        self.history.push_back(current);
                    }
                    self.current = Some(next);
                }
                self.current
            }
        }
    }

    pub fn previous(&mut self) -> Option<i64> {
        if let Some(prev) = self.history.pop_back() {
            if let Some(current) = self.current.take() {
                self.upcoming.push_front(current);
            }
            self.current = Some(prev);
        }
        self.current
    }

    pub fn current_id(&self) -> Option<i64> {
        self.current
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.upcoming.clear();
        self.current = None;
    }

    pub fn shuffle(&mut self) {
        let mut tracks: Vec<i64> = self.upcoming.drain(..).collect();
        tracks.shuffle(&mut rand::rng());
        self.upcoming = VecDeque::from(tracks);
    }

    pub fn upcoming(&self) -> &VecDeque<i64> {
        &self.upcoming
    }

    pub fn history(&self) -> &VecDeque<i64> {
        &self.history
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