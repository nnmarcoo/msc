use std::path::Path;

use blake3::Hash;
use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

pub struct Metadata {
    pub art_id: Option<Hash>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub duration: f32,
}

impl Metadata {
    pub fn from_path(path: &Path) -> Result<Self, LoftyError> {
        let file = Probe::open(path)?.read()?;
        let props = file.properties();
        let duration = props.duration().as_secs_f32();

        let (title, artist, album, genre) =
            if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
                (
                    tag.title().map(|s| s.to_string()),
                    tag.artist().map(|s| s.to_string()),
                    tag.album().map(|s| s.to_string()),
                    tag.genre().map(|s| s.to_string()),
                )
            } else {
                (None, None, None, None)
            };

        Ok(Metadata {
            title,
            artist,
            album,
            genre,
            duration,
            art_id: None,
        })
    }

    pub fn title_or_default(&self) -> &str {
        self.title.as_deref().unwrap_or("-")
    }

    pub fn artist_or_default(&self) -> &str {
        self.artist.as_deref().unwrap_or("-")
    }

    pub fn album_or_default(&self) -> &str {
        self.album.as_deref().unwrap_or("-")
    }

    pub fn genre_or_default(&self) -> &str {
        self.genre.as_deref().unwrap_or("-")
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }
}
