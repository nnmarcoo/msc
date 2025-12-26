use std::path::PathBuf;

use blake3::Hash;

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
}
