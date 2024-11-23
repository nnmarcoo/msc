use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use super::track::Track;

#[derive(Serialize, Deserialize, Debug)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    pub name: String,
}

impl Playlist {
    pub fn new() -> Self {
        Playlist {
            tracks: Vec::new(),
            name: String::from(""),
        }
    }

    pub fn from_directory(path: &str) -> Playlist {
        let tracks = Self::collect_audio_files(Path::new(path));
        Playlist {
            tracks,
            name: String::from(""),
        }
    }

    fn collect_audio_files(dir: &Path) -> Vec<Track> {
        if let Ok(entries) = read_dir(dir) {
            let entries: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();

            entries
                .par_iter()
                .flat_map(|path| {
                    if path.is_file() {
                        if let Some(extension) = path.extension() {
                            let ext = extension.to_string_lossy().to_lowercase();
                            if ["mp3", "flac", "m4a", "ogg"].contains(&ext.as_str()) {
                                if let Some(path_str) = path.to_str() {
                                    return vec![Track::new(path_str)].into_par_iter();
                                }
                            }
                        }
                    } else if path.is_dir() {
                        return Self::collect_audio_files(path).into_par_iter();
                    }
                    Vec::new().into_par_iter()
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
