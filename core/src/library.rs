use std::{
    error::Error,
    fmt::{self, Display},
    fs::read_dir,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{ArtCache, Database, Track};

pub struct Library {
    db: Database,
    pub art: Arc<ArtCache>,
    root: Option<PathBuf>,
}

impl Library {
    pub fn new(db_path: Option<&Path>) -> Result<Self, LibraryError> {
        let db = Database::new(db_path)?;
        Ok(Library {
            db,
            art: Arc::new(ArtCache::new()),
            root: None,
        })
    }

    pub fn populate(&mut self, root: &Path) -> Result<(), LibraryError> {
        self.root = Some(root.to_path_buf());
        self.reload()
    }

    pub fn reload(&mut self) -> Result<(), LibraryError> {
        if let Some(root) = &self.root {
            self.db.mark_all_missing()?;
            Self::scan_directory(&self.db, root)?;

            Ok(())
        } else {
            Err(LibraryError::RootNotSet)
        }
    }

    fn scan_directory(db: &Database, dir: &Path) -> Result<(), LibraryError> {
        if let Ok(entries) = read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if ["mp3", "flac", "wav", "ogg", "m4a", "aac"].contains(&ext.as_str()) {
                            if let Ok(track) = Track::from_path(&path) {
                                db.upsert_track(&track)?;
                                if let Some(path_str) = track.path.to_str() {
                                    db.mark_not_missing(path_str)?;
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    Self::scan_directory(db, &path)?;
                }
            }
        }
        Ok(())
    }

    pub fn track_from_id(&self, id: i64) -> Result<Option<Track>, LibraryError> {
        Ok(self.db.get_track_by_id(id)?)
    }

    pub fn all_tracks(&self) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_all_tracks()?)
    }

    pub fn tracks_by_album(&self, album_name: &str) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_tracks_by_album(album_name)?)
    }

    pub fn tracks_by_artist(&self, artist_name: &str) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_tracks_by_artist(artist_name)?)
    }

    pub fn track_count(&self) -> Result<i64, LibraryError> {
        Ok(self.db.count_tracks()?)
    }

    pub fn is_loaded(&self) -> bool {
        self.root.is_some()
    }

    pub fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }
}

#[derive(Debug)]
pub enum LibraryError {
    RootNotSet,
    Database(rusqlite::Error),
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LibraryError::RootNotSet => write!(f, "Library root directory not set"),
            LibraryError::Database(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl Error for LibraryError {}

impl From<rusqlite::Error> for LibraryError {
    fn from(err: rusqlite::Error) -> Self {
        LibraryError::Database(err)
    }
}
