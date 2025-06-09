use std::{borrow::Cow, path::Path};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub duration: f32,
}

impl Track {
    pub fn default() -> Self {
        Track {
            file_path: String::new(),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            genre: "Genre".to_string(),
            duration: 0.,
        }
    }

    pub fn new(path: &str) -> Option<Self> {
        let tagged_file = Probe::open(path).ok()?.read().ok()?;

        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        let properties = tagged_file.properties();
        let duration = properties.duration().as_secs_f32();

        let fallback_title = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Unknown");

        let (title, artist, album, genre) = if let Some(tag) = tag {
            (
                tag.title()
                    .unwrap_or_else(|| Cow::Borrowed(fallback_title))
                    .to_string(),
                tag.artist()
                    .unwrap_or_else(|| Cow::Borrowed("Unknown"))
                    .to_string(),
                tag.album()
                    .unwrap_or_else(|| Cow::Borrowed("Unknown"))
                    .to_string(),
                tag.genre()
                    .unwrap_or_else(|| Cow::Borrowed("Unknown"))
                    .to_string(),
            )
        } else {
            (
                fallback_title.to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
            )
        };

        Some(Track {
            file_path: path.to_string(),
            title,
            artist,
            album,
            genre,
            duration,
        })
    }
}
