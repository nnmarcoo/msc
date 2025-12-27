use std::{path::PathBuf, sync::Arc};

use blake3::Hash;
use dashmap::DashMap;

use crate::track::Track;

#[derive(Clone)]
enum PathOrhash {
    Path(PathBuf),
    Hash(Hash),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CollectionType {
    Album,
    Playlist,
}

#[derive(Clone)]
pub struct Collection {
    pub id: Hash,
    pub name: String,
    pub artist: Option<String>,
    pub collection_type: CollectionType,
    image: PathOrhash,
    tracks: Vec<Hash>,
}

impl Collection {
    pub fn from_album(
        id: Hash,
        name: String,
        artist: Option<String>,
        tracks: Vec<Hash>,
    ) -> Option<Self> {
        if tracks.is_empty() {
            return None;
        }

        let first_hash = tracks[0];

        Some(Collection {
            id,
            name,
            artist,
            collection_type: CollectionType::Album,
            image: PathOrhash::Hash(first_hash),
            tracks,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tracks(&self) -> &[Hash] {
        &self.tracks
    }

    pub fn image_hash(&self) -> Option<Hash> {
        match &self.image {
            PathOrhash::Hash(hash) => Some(*hash),
            PathOrhash::Path(_) => None,
        }
    }

    pub fn id(&self) -> Hash {
        self.id
    }

    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    pub fn collection_type(&self) -> CollectionType {
        self.collection_type
    }

    pub fn add_track(&mut self, track_hash: Hash) {
        if !self.tracks.contains(&track_hash) {
            self.tracks.push(track_hash);
        }
    }

    pub fn sort_tracks(&mut self, tracks_map: &DashMap<Hash, Arc<Track>>) {
        self.tracks.sort_by(|a, b| {
            let track_a = tracks_map.get(a);
            let track_b = tracks_map.get(b);

            match (track_a, track_b) {
                (Some(ta), Some(tb)) => {
                    let track_num_a = ta.metadata.track.unwrap_or(u32::MAX);
                    let track_num_b = tb.metadata.track.unwrap_or(u32::MAX);
                    track_num_a.cmp(&track_num_b)
                }
                _ => std::cmp::Ordering::Equal,
            }
        });
    }
}
