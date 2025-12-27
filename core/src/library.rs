use std::{
    error::Error,
    fmt::{self, Display},
    fs::read_dir,
    path::{Path, PathBuf},
    sync::Arc,
};

use blake3::Hash;
use dashmap::DashMap;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{ArtCache, collection::Collection, track::Track};

pub struct Library {
    pub tracks: DashMap<Hash, Arc<Track>>,
    pub collections: DashMap<Hash, Arc<Collection>>,
    pub art: Arc<ArtCache>,
    root: Option<PathBuf>,
}

impl Library {
    pub fn new() -> Self {
        Library {
            tracks: DashMap::new(),
            collections: DashMap::new(),
            art: Arc::new(ArtCache::new()),
            root: None,
        }
    }

    pub fn populate(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        let _ = self.reload();
    }

    pub fn reload(&mut self) -> Result<(), LibraryError> {
        if let Some(root) = &self.root {
            let tracks_map = DashMap::new();
            Library::collect_into(&root, &tracks_map);

            // Build collections by grouping tracks by art_id
            let collection_data: DashMap<Hash, (String, Option<String>, Vec<Hash>)> =
                DashMap::new();

            for entry in tracks_map.iter() {
                let (track_id, track) = entry.pair();
                if let Some(art_id) = track.metadata.art_id {
                    collection_data
                        .entry(art_id)
                        .and_modify(|(_, _, tracks)| tracks.push(*track_id))
                        .or_insert_with(|| {
                            let album_name = track.metadata.album_or_default();
                            let artist = track
                                .metadata
                                .track_artist
                                .clone()
                                .or_else(|| track.metadata.album_artist.clone());
                            (album_name, artist, vec![*track_id])
                        });
                }
            }

            // Create sorted collections
            let collections_map = DashMap::new();
            for entry in collection_data {
                let (art_id, (name, artist, mut tracks)) = entry;

                // Sort tracks by track number
                tracks.sort_by(|a, b| {
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

                let collection = Collection::new(
                    art_id,
                    name,
                    artist,
                    crate::collection::CollectionType::Album,
                    tracks,
                );
                collections_map.insert(art_id, Arc::new(collection));
            }

            self.tracks = tracks_map;
            self.collections = collections_map;
            Ok(())
        } else {
            Err(LibraryError::RootNotSet)
        }
    }

    fn collect_into(dir: &Path, tracks_map: &DashMap<Hash, Arc<Track>>) {
        if let Ok(entries) = read_dir(dir) {
            entries.par_bridge().for_each(|res| {
                if let Ok(entry) = res {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let ext = ext.to_lowercase();
                            if ["mp3", "flac", "wav", "ogg"].contains(&ext.as_str()) {
                                if let Ok(track) = Track::from_path(&path) {
                                    tracks_map.insert(track.id, Arc::new(track));
                                }
                            }
                        }
                    } else if path.is_dir() {
                        Library::collect_into(&path, tracks_map);
                    }
                }
            });
        }
    }

    pub fn track_from_id(&self, id: Hash) -> Option<Arc<Track>> {
        self.tracks.get(&id).map(|track_ref| Arc::clone(&track_ref))
    }

    pub fn collection_from_id(&self, id: Hash) -> Option<Arc<Collection>> {
        self.collections
            .get(&id)
            .map(|collection_ref| Arc::clone(&collection_ref))
    }

    pub fn is_loaded(&self) -> bool {
        self.root.is_some()
    }

    pub fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }
}

#[derive(Debug)]
pub enum LibraryError {
    RootNotSet,
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LibraryError::RootNotSet => write!(f, "Library root directory not set"),
        }
    }
}

impl Error for LibraryError {}
