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

use crate::{ArtCache, track::Track};

pub struct Library {
    pub tracks: Option<DashMap<Hash, Track>>,
    pub art: Arc<ArtCache>,
    root: Option<PathBuf>,
}

impl Library {
    pub fn new() -> Self {
        Library {
            tracks: None,
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
            self.tracks = Some(Library::collect(&root));
            Ok(())
        } else {
            Err(LibraryError::RootNotSet)
        }
    }

    pub fn collect(dir: &Path) -> DashMap<Hash, Track> {
        let map = DashMap::new();

        if let Ok(entries) = read_dir(dir) {
            entries.par_bridge().for_each(|res| {
                if let Ok(entry) = res {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let ext = ext.to_lowercase();
                            if ["mp3", "flac", "wav", "ogg"].contains(&ext.as_str()) {
                                if let Ok(track) = Track::from_path(&path) {
                                    map.insert(track.id, track);
                                }
                            }
                        }
                    } else if path.is_dir() {
                        let sub_map = Library::collect(&path);
                        for (hash, track) in sub_map {
                            map.insert(hash, track);
                        }
                    }
                }
            });
        }
        map
    }

    pub fn track_from_id(&self, id: Hash) -> Option<Track> {
        let tracks = self.tracks.as_ref()?;
        tracks.get(&id).map(|track_ref| track_ref.clone())
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
