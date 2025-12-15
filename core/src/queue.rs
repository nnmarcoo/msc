use blake3::Hash;
use rand::seq::SliceRandom;
use std::collections::VecDeque;

pub struct Queue {
    history: VecDeque<Hash>,
    current: Option<Hash>,
    upcoming: VecDeque<Hash>,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            history: VecDeque::new(),
            current: None,
            upcoming: VecDeque::new(),
        }
    }

    pub fn add(&mut self, track_id: Hash) {
        if self.current.is_none() {
            self.current = Some(track_id);
        } else {
            self.upcoming.push_back(track_id);
        }
    }

    pub fn next(&mut self) -> Option<Hash> {
        if let Some(current) = self.current.take() {
            self.history.push_back(current);
        }
        self.current = self.upcoming.pop_front();
        self.current
    }

    pub fn previous(&mut self) -> Option<Hash> {
        if let Some(prev) = self.history.pop_back() {
            if let Some(current) = self.current.take() {
                self.upcoming.push_front(current);
            }
            self.current = Some(prev);
        }
        self.current
    }

    pub fn current_id(&self) -> Option<Hash> {
        self.current
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.upcoming.clear();
        self.current = None;
    }

    pub fn shuffle(&mut self) {
        let mut tracks: Vec<Hash> = self.upcoming.drain(..).collect();
        tracks.shuffle(&mut rand::rng());
        self.upcoming = VecDeque::from(tracks);
    }

    pub fn upcoming(&self) -> &VecDeque<Hash> {
        &self.upcoming
    }

    pub fn history(&self) -> &VecDeque<Hash> {
        &self.history
    }
}
