use std::path::{Path, PathBuf};
use thiserror::Error;

use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

#[derive(Debug, Clone)]
pub struct Track {
    pub(crate) id: Option<i64>,

    pub(crate) path: PathBuf,
    pub(crate) missing: bool,

    pub(crate) title: Option<String>,
    pub(crate) track_artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) album_artist: Option<String>,
    pub(crate) genre: Option<String>,
    pub(crate) year: Option<u32>,
    pub(crate) track_number: Option<u32>,
    pub(crate) disc_number: Option<u32>,
    pub(crate) comment: Option<String>,

    pub(crate) duration: f32,
    pub(crate) bit_rate: Option<u32>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) bit_depth: Option<u8>,
    pub(crate) channels: Option<u8>,
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
                tag.title().map(|s| s.into()),
                tag.artist().map(|s| s.into()),
                tag.album().map(|s| s.into()),
                tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
                    .map(|s| s.into()),
                tag.genre().map(|s| s.into()),
                tag.year(),
                tag.track(),
                tag.disk(),
                tag.comment().map(|s| s.into()),
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

    pub fn path(&self) -> &Path {
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
}

#[derive(Debug, Error)]
pub enum TrackError {
    #[error("Lofty error: {0}")]
    Lofty(#[from] LoftyError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
