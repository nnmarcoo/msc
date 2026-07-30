//! A single audio file and the metadata read from its tags.
//!
//! Ratings are owned by the user rather than the file: they are seeded from
//! whatever the tags contain the first time a file is scanned, then never
//! overwritten by a later rescan.

use std::path::{Path, PathBuf};
use thiserror::Error;

use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

pub const MIN_STARS: u8 = 1;
pub const MAX_STARS: u8 = 5;

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

    pub(crate) rating: Option<u8>,
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
                tag.title().map(std::convert::Into::into),
                tag.artist().map(std::convert::Into::into),
                tag.album().map(std::convert::Into::into),
                tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
                    .map(std::convert::Into::into),
                tag.genre().map(std::convert::Into::into),
                tag.year(),
                tag.track(),
                tag.disk(),
                tag.comment().map(std::convert::Into::into),
            )
        } else {
            (None, None, None, None, None, None, None, None, None)
        };

        let rating = file
            .primary_tag()
            .or_else(|| file.first_tag())
            .and_then(rating_from_tag);

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
            rating,
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

    pub fn available(&self) -> bool {
        !self.missing
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

    pub fn rating(&self) -> Option<u8> {
        self.rating
    }
}

pub fn stars_in_range(stars: u8) -> bool {
    (MIN_STARS..=MAX_STARS).contains(&stars)
}

fn rating_from_tag(tag: &lofty::tag::Tag) -> Option<u8> {
    use lofty::tag::{ItemKey, ItemValue};

    tag.items()
        .filter(|item| item.key() == &ItemKey::Popularimeter)
        .find_map(|item| match item.value() {
            ItemValue::Binary(bytes) => stars_from_id3_popm(bytes),
            ItemValue::Text(text) | ItemValue::Locator(text) => {
                stars_from_numeric_rating(text.trim().parse::<f32>().ok()?)
            }
        })
}

fn stars_from_id3_popm(popm: &[u8]) -> Option<u8> {
    let email_terminator = popm.iter().position(|&b| b == 0)?;
    let byte = *popm.get(email_terminator + 1)?;

    Some(match byte {
        0 => return None,
        1..=31 => 1,
        32..=95 => 2,
        96..=159 => 3,
        160..=223 => 4,
        _ => 5,
    })
}

fn stars_from_numeric_rating(value: f32) -> Option<u8> {
    const PERCENT_PER_STAR: f32 = 20.0;

    let stars = if value <= f32::from(MAX_STARS) {
        value
    } else {
        value / PERCENT_PER_STAR
    };

    let stars = stars.round();
    (stars >= f32::from(MIN_STARS) && stars <= f32::from(MAX_STARS)).then_some(stars as u8)
}

#[derive(Debug, Error)]
pub enum TrackError {
    #[error("Lofty error: {0}")]
    Lofty(#[from] LoftyError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
