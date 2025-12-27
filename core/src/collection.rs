use blake3::Hash;

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
    pub image_hash: Option<Hash>,
    pub tracks: Vec<Hash>,
}

impl Collection {
    pub fn new(
        id: Hash,
        name: String,
        artist: Option<String>,
        collection_type: CollectionType,
        tracks: Vec<Hash>,
    ) -> Self {
        let image_hash = tracks.first().copied();

        Collection {
            id,
            name,
            artist,
            collection_type,
            image_hash,
            tracks,
        }
    }
}
