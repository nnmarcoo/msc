use super::track::Track;

pub struct Queue {
    tracks: Vec<Track>,
}

impl Queue {
    pub fn new() -> Self {
        Queue {
            tracks: Vec::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Track> {
        self.tracks.iter_mut()
    }
}
