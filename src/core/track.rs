use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f32,
}

impl Track {
    pub fn default() -> Self {
        Track {
            file_path: String::new(),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration: 0.,
        }
    }

    pub fn new(path: &str) -> Option<Self> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag();

        let title = tag
            .and_then(|t| t.get_string(&ItemKey::TrackTitle).map(String::from))
            .or_else(|| {
                Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| "Unknown Title".to_string());

        let artist = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumArtist).map(String::from))
            .or_else(|| tag.and_then(|t| t.get_string(&ItemKey::TrackArtist).map(String::from)))
            .unwrap_or_else(|| "Unknown Artist".to_string());

        let album = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumTitle).map(String::from))
            .unwrap_or_else(|| "Unknown Album".to_string());

        let duration = properties.duration().as_secs_f32();

        Some(Track {
            file_path: path.to_string(),
            title,
            artist,
            album,
            duration,
        })
    }
}
