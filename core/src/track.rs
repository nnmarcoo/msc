use std::{
    error::Error,
    fmt::Display,
    io,
    path::{Path, PathBuf},
};

use blake3::{Hash, hash};
use lofty::{
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};

impl From<LoftyError> for TrackError {
    fn from(err: LoftyError) -> Self {
        TrackError::Lofty(err)
    }
}

impl From<io::Error> for TrackError {
    fn from(err: io::Error) -> Self {
        TrackError::Io(err)
    }
}

#[derive(Clone)]
pub struct Track {
    pub id: Option<i64>,

    pub path: PathBuf,
    pub missing: bool,

    pub title: Option<String>,
    pub track_artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub comment: Option<String>,

    pub duration: f32,
    pub bit_rate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,

    // TODO
    pub art_id: Option<Hash>,
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

        let (title, track_artist, album, album_artist, genre, year, track_number, disc_number, comment, art_id) =
            if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
                let art_hash = tag.pictures().first().map(|pic| hash(pic.data()));

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
                    art_hash,
                )
            } else {
                (None, None, None, None, None, None, None, None, None, None)
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
            art_id,
        })
    }

    // these are ugly
    pub fn title_or_default(&self) -> String {
        self.title.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn track_artist_or_default(&self) -> String {
        self.track_artist.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn album_or_default(&self) -> String {
        self.album.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn album_artist_or_default(&self) -> String {
        self.album_artist.clone().unwrap_or_else(|| "-".to_string())
    }

    pub fn genre_or_default(&self) -> String {
        self.genre.clone().unwrap_or_else(|| "-".to_string())
    }
}

#[derive(Debug)]
pub enum TrackError {
    Lofty(LoftyError),
    Io(io::Error),
}

impl Display for TrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TrackError::Lofty(e) => write!(f, "Lofty error: {}", e),
            TrackError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl Error for TrackError {}
