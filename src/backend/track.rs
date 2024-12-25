use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    fs::read_dir,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f32,
}

impl Default for Track {
    fn default() -> Self {
        Track {
            file_path: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            duration: 0.,
        }
    }
}

impl Track {
    pub fn new(path: &str) -> Option<Self> {
        if let Ok(tagged_file) = Probe::open(path).unwrap().read() {
            let properties = tagged_file.properties();
            let tag = tagged_file.primary_tag();
            let file_name = Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let title = tag
                .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
                .unwrap_or(file_name.clone());

            let artist = tag
                .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
                .unwrap_or(String::new());

            let album = tag
                .and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from))
                .unwrap_or(String::new());

            let duration = properties.duration().as_secs_f32();

            Some(Track {
                file_path: path.to_string(),
                title,
                artist,
                album,
                duration,
            })
        } else {
            None
        }
    }

    pub fn collect_tracks(path: &str) -> HashMap<String, Track> {
        if let Ok(entries) = read_dir(Path::new(path)) {
            let entries: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();

        }
        HashMap::new()
    }
}
