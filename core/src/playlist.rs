//! User-authored track collections.
//!
//! Playlists are the one part of the library that cannot be rebuilt from the
//! filesystem, so they survive schema rebuilds and keep referencing tracks
//! whose files have gone missing.

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub cover_track_id: Option<i64>,
    pub track_ids: Vec<i64>,
}

impl Playlist {
    pub fn track_count(&self) -> usize {
        self.track_ids.len()
    }

    pub fn cover_track(&self) -> Option<i64> {
        self.cover_track_id
            .or_else(|| self.track_ids.first().copied())
    }
}
