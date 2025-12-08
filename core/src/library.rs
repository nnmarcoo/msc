use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use blake3::Hash;
use dashmap::DashMap;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::track::Track;

pub struct Library {
    pub tracks: Option<DashMap<Hash, Track>>,
    root: Option<PathBuf>,
}

impl Library {
    pub fn new() -> Self {
        Library {
            tracks: None,
            root: None,
        }
    }

    pub fn populate(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        self.reload();
    }

    pub fn reload(&mut self) {
        if let Some(root) = &self.root {
            self.tracks = Some(Library::collect(&root));
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
                                    map.insert(track.id(), track);
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
}
