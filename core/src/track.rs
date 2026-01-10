use std::path::{Path, PathBuf};
use thiserror::Error;

use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

#[derive(Clone)]
pub struct Track {
    id: Option<i64>,

    path: PathBuf,
    missing: bool,

    title: Option<String>,
    track_artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    genre: Option<String>,
    year: Option<u32>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    comment: Option<String>,

    duration: f32,
    bit_rate: Option<u32>,
    sample_rate: Option<u32>,
    bit_depth: Option<u8>,
    channels: Option<u8>,
}

impl Track {
    pub fn from_path(path: &Path) -> Result<Self, TrackError> {
        let file = Probe::open(path)?.read()?;
        let props = file.properties();
        let duration = props.duration().as_secs_f32();
        let bit_rate = props.audio_bitrate();
        let sample_rate = props.sample_rate();
        let bit_depth = props.bit_depth();
        let channels = props.channels();

        let (
            title,
            track_artist,
            album,
            album_artist,
            genre,
            year,
            track_number,
            disc_number,
            comment,
        ) = if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
            (
                tag.title().map(|s| s.to_string()),
                tag.artist().map(|s| s.to_string()),
                tag.album().map(|s| s.to_string()),
                tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
                    .map(|s| s.to_string()),
                tag.genre().map(|s| s.to_string()),
                tag.year(),
                tag.track(),
                tag.disk(),
                tag.comment().map(|s| s.to_string()),
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
        };

        Ok(Track {
            id: None,
            path: path.to_path_buf(),
            missing: false,
            title,
            track_artist,
            album,
            album_artist,
            genre,
            year,
            track_number,
            disc_number,
            comment,
            duration,
            bit_rate,
            sample_rate,
            bit_depth,
            channels,
        })
    }

    pub fn id(&self) -> Option<i64> {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn missing(&self) -> bool {
        self.missing
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn track_artist(&self) -> Option<&str> {
        self.track_artist.as_deref()
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_deref()
    }

    pub fn album_artist(&self) -> Option<&str> {
        self.album_artist.as_deref()
    }

    pub fn genre(&self) -> Option<&str> {
        self.genre.as_deref()
    }

    pub fn year(&self) -> Option<u32> {
        self.year
    }

    pub fn track_number(&self) -> Option<u32> {
        self.track_number
    }

    pub fn disc_number(&self) -> Option<u32> {
        self.disc_number
    }

    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn bit_rate(&self) -> Option<u32> {
        self.bit_rate
    }

    pub fn sample_rate(&self) -> Option<u32> {
        self.sample_rate
    }

    pub fn bit_depth(&self) -> Option<u8> {
        self.bit_depth
    }

    pub fn channels(&self) -> Option<u8> {
        self.channels
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_db(
        id: Option<i64>,
        path: PathBuf,
        missing: bool,
        title: Option<String>,
        track_artist: Option<String>,
        album: Option<String>,
        album_artist: Option<String>,
        genre: Option<String>,
        year: Option<u32>,
        track_number: Option<u32>,
        disc_number: Option<u32>,
        comment: Option<String>,
        duration: f32,
        bit_rate: Option<u32>,
        sample_rate: Option<u32>,
        bit_depth: Option<u8>,
        channels: Option<u8>,
    ) -> Self {
        Track {
            id,
            path,
            missing,
            title,
            track_artist,
            album,
            album_artist,
            genre,
            year,
            track_number,
            disc_number,
            comment,
            duration,
            bit_rate,
            sample_rate,
            bit_depth,
            channels,
        }
    }

    pub fn title_or_default(&self) -> &str {
        self.title.as_deref().unwrap_or("-")
    }

    pub fn track_artist_or_default(&self) -> &str {
        self.track_artist.as_deref().unwrap_or("-")
    }

    pub fn album_or_default(&self) -> &str {
        self.album.as_deref().unwrap_or("-")
    }

    pub fn album_artist_or_default(&self) -> &str {
        self.album_artist.as_deref().unwrap_or("-")
    }

    pub fn genre_or_default(&self) -> &str {
        self.genre.as_deref().unwrap_or("-")
    }
}

#[derive(Debug, Error)]
pub enum TrackError {
    #[error("Lofty error: {0}")]
    Lofty(#[from] LoftyError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
