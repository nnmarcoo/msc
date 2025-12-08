use std::{io, path::PathBuf};

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

pub struct Track {
    id: Hash,  
    path: PathBuf,
    data: Metadata,
}

impl Track {
    pub fn from_path(path: PathBuf) -> Result<Self, TrackError> {
        let mut hasher = Hasher::new(); 
        let id = hasher.update_mmap(&path)?.finalize();

        let data = Metadata::from_path(&path)?;

        Ok(Track { id, path, data })
    }
}
