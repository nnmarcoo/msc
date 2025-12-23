use std::path::Path;

use blake3::{Hash, hash};
use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

#[derive(Clone)]
pub struct Metadata {
    pub art_id: Option<Hash>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub duration: f32,
    pub bit_rate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
}

impl Metadata {
    pub fn from_path(path: &Path) -> Result<Self, LoftyError> {
        let file = Probe::open(path)?.read()?;
        let props = file.properties();
        let duration = props.duration().as_secs_f32();
        let bit_rate = props.audio_bitrate();
        let sample_rate = props.sample_rate();
        let bit_depth = props.bit_depth();
        let channels = props.channels();

        let (title, artist, album, genre, art_id) =
            if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
                let art_hash = tag.pictures().first().map(|pic| hash(pic.data()));

                (
                    tag.title().map(|s| s.to_string()),
                    tag.artist().map(|s| s.to_string()),
                    tag.album().map(|s| s.to_string()),
                    tag.genre().map(|s| s.to_string()),
                    art_hash,
                )
            } else {
                (None, None, None, None, None)
            };

        Ok(Metadata {
            title,
            artist,
            album,
            genre,
            duration,
            art_id,
            bit_rate,
            sample_rate,
            bit_depth,
            channels,
        })
    }

    pub fn title_or_default(&self) -> String {
        self.title.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn artist_or_default(&self) -> String {
        self.artist.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn album_or_default(&self) -> String {
        self.album.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn genre_or_default(&self) -> String {
        self.genre.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }
}
