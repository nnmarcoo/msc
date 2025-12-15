use std::path::PathBuf;

use blake3::Hash;

use crate::Library;

enum PathOrhash {
    Path(PathBuf),
    Hash(Hash),
}

pub struct Collection {
    name: String,
    image: PathOrhash,
    tracks: Vec<Hash>,
}

impl Collection {
    pub fn from_tracks(library: &Library, track_hashes: Vec<Hash>) -> Option<Self> {
        if track_hashes.is_empty() {
            return None;
        }

        let first_hash = track_hashes[0];
        let first_track = library.track_from_id(first_hash)?;
        let album_name = first_track.metadata.album_or_default().to_string();

        Some(Collection {
            name: album_name,
            image: PathOrhash::Hash(first_hash),
            tracks: track_hashes,
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
}
