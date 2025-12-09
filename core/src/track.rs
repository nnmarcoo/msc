use std::{
    fs, io,
    path::{Path, PathBuf},
};

use blake3::{Hash, Hasher};
use lofty::error::LoftyError;

use crate::Metadata;

#[derive(Debug)]
pub enum TrackError {
    Lofty(LoftyError),
    Io(io::Error),
}

impl std::fmt::Display for TrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TrackError::Lofty(e) => write!(f, "Lofty error: {}", e),
            TrackError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for TrackError {}

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
    pub id: Hash,
    pub path: PathBuf,
    pub metadata: Metadata,
}

impl Track {
    pub fn from_path(path: &Path) -> Result<Self, TrackError> {
        let data = Metadata::from_path(path)?;

        let id = {
            let mut hasher = Hasher::new();
            hasher.update(data.artist_or_default().as_bytes());
            hasher.update(data.title_or_default().as_bytes());
            hasher.update(data.album_or_default().as_bytes());
            hasher.update(data.genre_or_default().as_bytes());
            hasher.update(&data.duration().to_le_bytes());
            hasher.update(&fs::metadata(path)?.len().to_le_bytes());
            hasher.finalize()
        };

        Ok(Track {
            id,
            path: path.to_path_buf(),
            metadata: data,
        })
    }
}
