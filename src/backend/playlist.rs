use super::track::Track;

pub struct Playlist {
  tracks: Vec<Track>
}

impl Playlist {
    pub fn new() -> Self {
      Playlist {
        tracks: Vec::new(),
      }
    }
}