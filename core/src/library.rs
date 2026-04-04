use rayon::prelude::*;
use std::{fs::create_dir_all, path::Path};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{Album, Config, ConfigError, Database, Track};

pub struct Library {
    db: Database,
}

impl Library {
    pub fn new() -> Result<Self, LibraryError> {
        let db_path = Config::database_path()?;

        if let Some(parent) = db_path.parent() {
            create_dir_all(parent)?;
        }

        Ok(Library {
            db: Database::new(&db_path)?,
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

    fn scan_directory(db: &Database, root: &Path) -> Result<(), LibraryError> {
        const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

        let audio_files: Vec<_> = WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .flatten()
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                        .unwrap_or(false)
            })
            .map(|e| e.into_path())
            .collect();

        let tracks: Vec<Track> = audio_files
            .par_iter()
            .filter_map(|path| Track::from_path(path).ok())
            .collect();

        db.batch_upsert_tracks(&tracks)?;
        db.batch_upsert_albums_from_tracks(&tracks)?;

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

    pub fn query_all_albums(&self) -> Result<Vec<Album>, LibraryError> {
        Ok(self.db.get_all_albums()?)
    }

    pub fn query_track_from_path(&self, path: &str) -> Result<Option<Track>, LibraryError> {
        Ok(self.db.get_track_by_path(path)?)
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
