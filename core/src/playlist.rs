/// User-authored, and the one thing in the library that cannot be rebuilt from
/// the filesystem — so playlists survive schema rebuilds and missing files.
#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    /// Explicit cover choice. When unset, consumers fall back to the first
    /// track's artwork.
    pub cover_track_id: Option<i64>,
    /// Track ids in playlist order. Ids of missing tracks are retained.
    pub track_ids: Vec<i64>,
}

impl Playlist {
    pub fn track_count(&self) -> usize {
        self.track_ids.len()
    }

    /// The explicit cover if set, else the first track.
    pub fn cover_track(&self) -> Option<i64> {
        self.cover_track_id
            .or_else(|| self.track_ids.first().copied())
    }
}
