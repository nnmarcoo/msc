use std::{
    fs::{create_dir_all, read_dir},
    path::Path,
    sync::Arc,
};
use thiserror::Error;

use crate::{ArtCache, Colors, Config, ConfigError, Database, RgbaImage, Track};

pub struct Library {
    db: Database,
    art: Arc<ArtCache>,
}

impl Library {
    pub fn new() -> Result<Self, LibraryError> {
        let db_path = Config::database_path()?;

        if let Some(parent) = db_path.parent() {
            create_dir_all(parent)?;
        }

        let db = Database::new(&db_path)?;
        Ok(Library {
            db,
            art: Arc::new(ArtCache::new()),
        })
    }

    pub fn populate(&mut self, root: &Path) -> Result<(), LibraryError> {
        Config::set_root(root.to_path_buf())?;
        self.reload()
    }

    pub fn reload(&mut self) -> Result<(), LibraryError> {
        if let Some(root) = Config::root() {
            self.db.mark_all_missing()?;
            Self::scan_directory(&self.db, &root)?;

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
                                if let Some(path_str) = track.path().to_str() {
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

    pub fn query_track_from_id(&self, id: i64) -> Result<Option<Track>, LibraryError> {
        Ok(self.db.get_track_by_id(id)?)
    }

    pub fn query_all_tracks(&self) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_all_tracks()?)
    }

    pub fn query_n_tracks(&self, limit: i64) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_n_tracks(limit)?)
    }

    pub fn query_tracks_by_album(&self, album_name: &str) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_tracks_by_album(album_name)?)
    }

    pub fn query_tracks_by_artist(&self, artist_name: &str) -> Result<Vec<Track>, LibraryError> {
        Ok(self.db.get_tracks_by_artist(artist_name)?)
    }

    pub fn query_track_count(&self) -> Result<i64, LibraryError> {
        Ok(self.db.count_tracks()?)
    }

    pub fn artwork(&self, track: &Track) -> Option<(RgbaImage, Colors)> {
        self.art.get(track)
    }
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("Library root directory not set")]
    RootNotSet,
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
