use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{Album, Playlist, Track, album, db::Database, track};

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

pub struct Library {
    tracks: Vec<Track>,
    by_id: HashMap<i64, usize>,
    by_path: HashMap<PathBuf, usize>,
    albums: Vec<Album>,
    playlists: Vec<Playlist>,
    root: Option<PathBuf>,
    db: Database,
}

impl Library {
    pub fn open() -> Result<Self, LibraryError> {
        let path = database_path()?;
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        let db = Database::open(&path)?;
        let mut library = Library {
            tracks: Vec::new(),
            by_id: HashMap::new(),
            by_path: HashMap::new(),
            albums: Vec::new(),
            playlists: Vec::new(),
            root: None,
            db,
        };
        library.reload()?;
        Ok(library)
    }

    fn reload(&mut self) -> Result<(), LibraryError> {
        self.tracks = self.db.all_tracks()?;
        self.root = self.db.get_meta("root")?.map(PathBuf::from);

        self.by_id = self
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.id().map(|id| (id, i)))
            .collect();
        self.by_path = self
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.path().to_path_buf(), i))
            .collect();

        self.albums = album::derive(&self.tracks);

        let mut membership: HashMap<i64, Vec<i64>> = HashMap::new();
        for (playlist_id, track_id) in self.db.all_playlist_tracks()? {
            membership.entry(playlist_id).or_default().push(track_id);
        }
        self.playlists = self
            .db
            .all_playlists()?
            .into_iter()
            .map(|(id, name, cover_track_id)| Playlist {
                track_ids: membership.remove(&id).unwrap_or_default(),
                id,
                name,
                cover_track_id,
            })
            .collect();

        Ok(())
    }
}

impl Library {
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn available(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| !t.missing())
    }

    pub fn track(&self, id: i64) -> Option<&Track> {
        self.by_id.get(&id).map(|&i| &self.tracks[i])
    }

    pub fn track_by_path(&self, path: &Path) -> Option<&Track> {
        self.by_path.get(path).map(|&i| &self.tracks[i])
    }

    pub fn albums(&self) -> &[Album] {
        &self.albums
    }

    pub fn album_tracks(&self, album: &Album) -> impl Iterator<Item = &Track> {
        album.track_indices.iter().map(|&i| &self.tracks[i])
    }

    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists
    }

    pub fn playlist(&self, id: i64) -> Option<&Playlist> {
        self.playlists.iter().find(|p| p.id == id)
    }

    pub fn playlist_tracks(&self, id: i64) -> impl Iterator<Item = &Track> {
        self.playlist(id)
            .into_iter()
            .flat_map(|p| p.track_ids.iter().filter_map(|&id| self.track(id)))
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

impl Library {
    pub fn scan(&mut self, root: &Path) -> Result<(), LibraryError> {
        let files: Vec<PathBuf> = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file() && is_audio(e.path()))
            .map(walkdir::DirEntry::into_path)
            .collect();

        let tracks: Vec<Track> = files
            .par_iter()
            .filter_map(|path| Track::from_path(path).ok())
            .collect();

        self.db.mark_all_missing()?;
        self.db.upsert_tracks(&tracks)?;
        self.db.set_meta("root", &root.to_string_lossy())?;
        self.db.relink_pending()?;

        self.reload()
    }

    pub fn rescan(&mut self) -> Result<(), LibraryError> {
        let root = self.root.clone().ok_or(LibraryError::RootNotSet)?;
        self.scan(&root)
    }

    pub fn clear(&mut self) -> Result<(), LibraryError> {
        self.db.clear_library()?;
        self.reload()
    }
}

impl Library {
    pub fn create_playlist(&mut self, name: &str) -> Result<i64, LibraryError> {
        let id = self.db.create_playlist(name)?;
        let position = self
            .playlists
            .iter()
            .position(|p| p.name.as_str() > name)
            .unwrap_or(self.playlists.len());
        self.playlists.insert(
            position,
            Playlist {
                id,
                name: name.to_owned(),
                cover_track_id: None,
                track_ids: Vec::new(),
            },
        );
        Ok(id)
    }

    pub fn rename_playlist(&mut self, id: i64, name: &str) -> Result<(), LibraryError> {
        self.db.rename_playlist(id, name)?;
        if let Some(p) = self.playlists.iter_mut().find(|p| p.id == id) {
            name.clone_into(&mut p.name);
        }
        self.playlists.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    pub fn delete_playlist(&mut self, id: i64) -> Result<(), LibraryError> {
        self.db.delete_playlist(id)?;
        self.playlists.retain(|p| p.id != id);
        Ok(())
    }

    pub fn add_to_playlist(&mut self, playlist_id: i64, track_id: i64) -> Result<(), LibraryError> {
        let Some(playlist) = self.playlists.iter_mut().find(|p| p.id == playlist_id) else {
            return Ok(());
        };
        if playlist.track_ids.contains(&track_id) {
            return Ok(());
        }
        self.db.add_track_to_playlist(
            playlist_id,
            track_id,
            i64::try_from(playlist.track_ids.len()).unwrap_or(i64::MAX),
        )?;
        playlist.track_ids.push(track_id);
        Ok(())
    }

    pub fn remove_from_playlist(
        &mut self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<(), LibraryError> {
        self.db.remove_track_from_playlist(playlist_id, track_id)?;
        if let Some(p) = self.playlists.iter_mut().find(|p| p.id == playlist_id) {
            p.track_ids.retain(|&id| id != track_id);
        }
        Ok(())
    }

    pub fn set_playlist_cover(
        &mut self,
        playlist_id: i64,
        track_id: Option<i64>,
    ) -> Result<(), LibraryError> {
        self.db.set_playlist_cover(playlist_id, track_id)?;
        if let Some(p) = self.playlists.iter_mut().find(|p| p.id == playlist_id) {
            p.cover_track_id = track_id;
        }
        Ok(())
    }

    pub fn set_rating(&mut self, track_id: i64, rating: Option<u8>) -> Result<(), LibraryError> {
        let rating = rating.filter(|&stars| track::stars_in_range(stars));
        self.db.set_rating(track_id, rating)?;
        if let Some(&index) = self.by_id.get(&track_id) {
            self.tracks[index].rating = rating;
        }
        Ok(())
    }

    pub fn rated(&self, min_stars: u8) -> impl Iterator<Item = &Track> {
        self.tracks
            .iter()
            .filter(move |t| t.rating().is_some_and(|r| r >= min_stars))
    }
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

fn database_path() -> Result<PathBuf, LibraryError> {
    directories::ProjectDirs::from("", "", "verse")
        .map(|dirs| dirs.data_dir().join("library.db"))
        .ok_or(LibraryError::DataDirNotFound)
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("Library root directory not set")]
    RootNotSet,
    #[error("Could not determine the data directory")]
    DataDirNotFound,
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
