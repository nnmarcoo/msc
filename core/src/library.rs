use rayon::prelude::*;
use std::{fs::create_dir_all, path::Path};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{Album, Config, ConfigError, Database, Playlist, Track};

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

    pub fn scan_with_root(root: &Path) -> Result<(), LibraryError> {
        Config::set_root(root.to_path_buf())?;
        Self::scan()
    }

    pub fn scan() -> Result<(), LibraryError> {
        let root = Config::root().ok_or(LibraryError::RootNotSet)?;
        let db_path = Config::database_path()?;

        if let Some(parent) = db_path.parent() {
            create_dir_all(parent)?;
        }

        let db = Database::new(&db_path)?;
        db.mark_all_missing()?;
        Self::scan_directory(&db, &root)?;
        Ok(())
    }

    fn scan_directory(db: &Database, root: &Path) -> Result<(), LibraryError> {
        const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

        let audio_files: Vec<_> = WalkDir::new(root)
            .follow_links(false)
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

    pub(crate) fn clear(&self) -> Result<(), LibraryError> {
        Ok(self.db.clear_library()?)
    }
}

/// Forwards read-only queries to the underlying [`Database`], mapping
/// `rusqlite::Error` into [`LibraryError`] so callers never see the SQL layer.
macro_rules! forward_queries {
    ($( $name:ident -> $ret:ty = $db_method:ident ( $( $arg:ident : $ty:ty ),* ); )*) => {
        impl Library {
            $(
                pub fn $name(&self, $( $arg: $ty ),*) -> Result<$ret, LibraryError> {
                    Ok(self.db.$db_method($( $arg ),*)?)
                }
            )*
        }
    };
}

forward_queries! {
    query_track_from_id -> Option<Track> = get_track_by_id(id: i64);
    query_track_from_path -> Option<Track> = get_track_by_path(path: &str);
    query_all_tracks -> Vec<Track> = get_all_tracks();
    query_n_tracks -> Vec<Track> = get_n_tracks(limit: i64);
    query_tracks_by_album -> Vec<Track> = get_tracks_by_album(album_name: &str, artist: Option<&str>);
    query_tracks_by_artist -> Vec<Track> = get_tracks_by_artist(artist_name: &str);
    query_track_count -> i64 = count_tracks();
    query_all_albums -> Vec<Album> = get_all_albums();
    get_all_playlists -> Vec<Playlist> = get_all_playlists();
    get_tracks_in_playlist -> Vec<Track> = get_tracks_in_playlist(playlist_id: i64);
}

/// Playlist mutations. These stay distinct from the query forwards because
/// `Player` wraps them to keep the queue consistent with the library.
impl Library {
    pub(crate) fn create_playlist(&self, name: &str) -> Result<i64, LibraryError> {
        Ok(self.db.create_playlist(name)?)
    }

    pub(crate) fn rename_playlist(&self, id: i64, name: &str) -> Result<(), LibraryError> {
        Ok(self.db.rename_playlist(id, name)?)
    }

    pub(crate) fn delete_playlist(&self, id: i64) -> Result<(), LibraryError> {
        Ok(self.db.delete_playlist(id)?)
    }

    pub(crate) fn add_track_to_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<(), LibraryError> {
        Ok(self.db.add_track_to_playlist(playlist_id, track_id)?)
    }

    pub(crate) fn remove_track_from_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<(), LibraryError> {
        Ok(self.db.remove_track_from_playlist(playlist_id, track_id)?)
    }

    pub(crate) fn set_playlist_cover(
        &self,
        playlist_id: i64,
        track_id: Option<i64>,
    ) -> Result<(), LibraryError> {
        Ok(self.db.set_playlist_cover(playlist_id, track_id)?)
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
