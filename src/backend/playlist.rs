use super::track::Track;

pub struct Playlist {
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist { tracks: Vec::new() }
    }

    pub fn to_string(&self) -> String {
        self.tracks
            .iter()
            .map(|track| track.to_string())
            .collect::<Vec<String>>()
            .join("\n\n")
    }
}
