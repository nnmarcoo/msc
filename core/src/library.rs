//! The in-memory library: every track, and the albums grouped from them.
//!
//! Tracks are loaded whole and filtered on the way out rather than in SQL, so
//! `tracks` holds everything the database knows and [`Library::available`] is
//! what a caller listing playable music asks for. [`Track::available`] is the
//! single definition of that rule; nothing outside should test `missing` itself,
//! or two views of the same library will disagree about what can be played.
//! [`Library::album_tracks_available`] applies the same rule within one album,
//! which is why an album with one missing track is still listed while one with
//! no playable tracks at all is not.
//!
//! [`Library::albums_by_key`] resolves keys back to albums in a single pass,
//! advancing through `albums` in step with the keys rather than searching for
//! each one. That requires the keys to arrive in `albums` order, which holds for
//! their only source: a filtered view of that same list. Keys in another order
//! resolve only the ones that happen to fall in sequence, so this is a lookup for
//! a caller narrowing the album list, not a general one. Callers holding
//! arbitrary keys want a scan per key, and should not use this.
//!
//! [`Library::album`] is that scan, and the one to reach for when resolving a
//! single key: it answers the same whatever order the keys arrive in, where
//! passing one key to [`Library::albums_by_key`] happens to work only because a
//! lone key cannot fall out of sequence with itself. Albums number in the tens
//! to hundreds and this runs on a click rather than per frame, so it stays a
//! scan rather than an index that `reload` would have to keep in step.
//!
//! A key that no longer matches any album is skipped rather than faulted, since
//! a rescan can retire an album while a cached view of it is still in flight.
//!
//! [`Library::ingest`] adds one file without walking the tree. It exists
//! because a track arriving on its own — downloaded, or dropped into the folder
//! — should appear at once, and [`Library::scan`] is the wrong tool for that in
//! two ways: it costs a full re-walk, and it opens by marking every track
//! missing, so a scan interrupted partway leaves the library claiming files it
//! never got to. Ingesting one file touches only that row.
//!
//! It deliberately does not set `root`. A file ingested from outside the library
//! folder is still playable and still listed, but it is not evidence about where
//! the collection lives, and letting one download silently repoint the root
//! would send the next rescan somewhere the user never chose.
//!
//! A rating already on the row survives, because the upsert leaves that column
//! alone; ingesting a file the library already holds refreshes its tags and
//! keeps what the user set. That is the same rule a rescan follows, for the same
//! reason.
//!
//! [`Library::ingest_many`] is the same operation for a batch, and exists
//! because the reload is the expensive half. Ingesting an album a track at a
//! time paid a full rebuild — every row, both indexes, every album — per track,
//! at a cost set by the size of the library rather than the size of the
//! download. Batching pays it once. A file that cannot be read is skipped rather
//! than failing the rest, and the caller sees which by what does not come back.
//!
//! [`Library::open`] takes the one database the application owns, which makes it
//! useless to a test: two tests running in the same process would share a file
//! and see each other's tracks. [`Library::open_at`] names the database instead,
//! so a test can hold its own in a temporary directory. It is public rather than
//! `pub(crate)` because the integration tests are a separate crate.

use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::WalkDir;

use crate::{Album, AlbumKey, Playlist, Track, album, db::Database, track};

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

        Self::open_at(&path)
    }

    pub fn open_at(path: &Path) -> Result<Self, LibraryError> {
        let db = Database::open(path)?;
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
        self.tracks.iter().filter(|t| t.available())
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

    pub fn album_tracks_available(&self, album: &Album) -> impl Iterator<Item = &Track> {
        self.album_tracks(album).filter(|t| t.available())
    }

    pub fn album(&self, key: &AlbumKey) -> Option<&Album> {
        album_by_key(&self.albums, key)
    }

    pub fn albums_by_key<'a>(
        &'a self,
        keys: impl IntoIterator<Item = &'a AlbumKey, IntoIter: ExactSizeIterator>,
    ) -> Vec<&'a Album> {
        albums_by_key(&self.albums, keys)
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
            .filter_entry(|e| !is_hidden_directory(e))
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

    pub fn ingest(&mut self, path: &Path) -> Result<i64, LibraryError> {
        if !is_audio(path) {
            return Err(LibraryError::NotAudio(path.display().to_string()));
        }

        let track = Track::from_path(path)
            .map_err(|e| LibraryError::Unreadable(path.display().to_string(), e))?;

        self.store(&[track])?;

        self.track_by_path(path)
            .and_then(Track::id)
            .ok_or_else(|| LibraryError::NotIngested(path.display().to_string()))
    }

    pub fn ingest_many(&mut self, paths: &[PathBuf]) -> Result<Vec<(PathBuf, i64)>, LibraryError> {
        let tracks: Vec<Track> = paths
            .par_iter()
            .filter(|path| is_audio(path))
            .filter_map(|path| Track::from_path(path).ok())
            .collect();

        if tracks.is_empty() {
            return Ok(Vec::new());
        }

        self.store(&tracks)?;

        Ok(paths
            .iter()
            .filter_map(|path| {
                let id = self.track_by_path(path).and_then(Track::id)?;
                Some((path.clone(), id))
            })
            .collect())
    }

    fn store(&mut self, tracks: &[Track]) -> Result<(), LibraryError> {
        self.db.upsert_tracks(tracks)?;
        self.db.relink_pending()?;
        self.reload()
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

fn album_by_key<'a>(albums: &'a [Album], key: &AlbumKey) -> Option<&'a Album> {
    albums.iter().find(|album| album.key == *key)
}

fn albums_by_key<'a>(
    albums: &'a [Album],
    keys: impl IntoIterator<Item = &'a AlbumKey, IntoIter: ExactSizeIterator>,
) -> Vec<&'a Album> {
    let keys = keys.into_iter();
    let mut resolved = Vec::with_capacity(keys.len());
    let mut next = 0;

    for key in keys {
        if let Some(offset) = albums[next..].iter().position(|album| album.key == *key) {
            let found = next + offset;
            resolved.push(&albums[found]);
            next = found + 1;
        }
    }

    resolved
}

fn is_hidden_directory(entry: &walkdir::DirEntry) -> bool {
    entry.depth() > 0
        && entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with('.'))
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

fn database_path() -> Result<PathBuf, LibraryError> {
    directories::ProjectDirs::from("", "", "verse")
        .map(|dirs| dirs.data_dir().join("library.db"))
        .ok_or(LibraryError::DataDirNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn album(name: &str) -> Album {
        Album {
            key: AlbumKey {
                name: name.to_owned(),
                artist: None,
            },
            year: None,
            track_indices: Vec::new(),
        }
    }

    fn names(albums: &[&Album]) -> Vec<String> {
        albums.iter().map(|a| a.name().to_owned()).collect()
    }

    #[test]
    fn a_filtered_view_resolves_to_the_albums_it_named() {
        let albums = [album("Blue Lines"), album("Mezzanine"), album("Protection")];
        let keys = [albums[1].key.clone(), albums[2].key.clone()];

        assert_eq!(
            names(&albums_by_key(&albums, &keys)),
            ["Mezzanine", "Protection"]
        );
    }

    #[test]
    fn resolving_stays_a_single_pass_however_sparse_the_view_is() {
        let albums: Vec<Album> = (0..20).map(|n| album(&format!("Album {n}"))).collect();
        let keys: Vec<AlbumKey> = albums.iter().step_by(7).map(|a| a.key.clone()).collect();

        assert_eq!(
            names(&albums_by_key(&albums, &keys)),
            ["Album 0", "Album 7", "Album 14"]
        );
    }

    #[test]
    fn every_album_resolves_when_nothing_was_filtered() {
        let albums: Vec<Album> = (0..8).map(|n| album(&format!("Album {n}"))).collect();
        let keys: Vec<AlbumKey> = albums.iter().map(|a| a.key.clone()).collect();

        assert_eq!(albums_by_key(&albums, &keys).len(), albums.len());
    }

    #[test]
    fn a_key_that_left_the_library_is_skipped_rather_than_shifting_the_rest() {
        let albums = [album("Blue Lines"), album("Protection")];
        let stale = [
            albums[0].key.clone(),
            AlbumKey {
                name: "Mezzanine".into(),
                artist: None,
            },
            albums[1].key.clone(),
        ];

        assert_eq!(
            names(&albums_by_key(&albums, &stale)),
            ["Blue Lines", "Protection"],
            "a rescanned-away album displaced the covers after it"
        );
    }

    #[test]
    fn keys_out_of_order_are_not_all_found() {
        let albums = [album("Blue Lines"), album("Mezzanine"), album("Protection")];
        let reversed = [albums[2].key.clone(), albums[0].key.clone()];

        assert_eq!(names(&albums_by_key(&albums, &reversed)), ["Protection"]);
    }

    #[test]
    fn no_keys_resolve_to_no_albums() {
        let albums = [album("Blue Lines")];
        assert!(albums_by_key(&albums, &[]).is_empty());
    }

    #[test]
    fn keys_against_an_empty_library_resolve_to_nothing() {
        let key = AlbumKey {
            name: "Blue Lines".into(),
            artist: None,
        };
        assert!(albums_by_key(&[], &[key]).is_empty());
    }

    #[test]
    fn a_key_resolves_wherever_it_sits_in_the_list() {
        let albums = [album("Blue Lines"), album("Mezzanine"), album("Protection")];

        for expected in &albums {
            assert_eq!(
                album_by_key(&albums, &expected.key).map(Album::name),
                Some(expected.name()),
                "a lookup depended on where the key sat"
            );
        }
    }

    #[test]
    fn a_lookup_does_not_care_what_order_the_keys_came_in() {
        let albums = [album("Blue Lines"), album("Mezzanine"), album("Protection")];
        let backwards = [albums[2].key.clone(), albums[0].key.clone()];

        let found: Vec<&str> = backwards
            .iter()
            .filter_map(|key| album_by_key(&albums, key))
            .map(Album::name)
            .collect();

        assert_eq!(found, ["Protection", "Blue Lines"]);
    }

    #[test]
    fn a_key_that_left_the_library_resolves_to_nothing() {
        let albums = [album("Blue Lines")];
        let gone = AlbumKey {
            name: "Mezzanine".into(),
            artist: None,
        };

        assert!(album_by_key(&albums, &gone).is_none());
    }

    #[test]
    fn albums_sharing_a_title_are_told_apart_by_their_artist() {
        let albums = [
            Album {
                key: AlbumKey {
                    name: "Greatest Hits".into(),
                    artist: Some("First".into()),
                },
                year: None,
                track_indices: vec![0],
            },
            Album {
                key: AlbumKey {
                    name: "Greatest Hits".into(),
                    artist: Some("Second".into()),
                },
                year: None,
                track_indices: vec![1],
            },
        ];

        assert_eq!(
            album_by_key(&albums, &albums[1].key).map(|a| a.track_indices.clone()),
            Some(vec![1]),
            "the credit is part of the key, so a shared title must not collide"
        );
    }

    #[test]
    fn every_supported_extension_is_recognized() {
        for extension in AUDIO_EXTENSIONS {
            let path = PathBuf::from(format!("song.{extension}"));
            assert!(is_audio(&path), "{extension} was not recognized as audio");
        }
    }

    #[test]
    fn an_extension_is_recognized_however_it_was_capitalized() {
        for name in ["song.MP3", "song.Flac", "song.WaV", "song.OGG"] {
            assert!(
                is_audio(Path::new(name)),
                "{name} was skipped by a scan for its capitalization"
            );
        }
    }

    #[test]
    fn a_file_that_is_not_audio_is_skipped() {
        for name in ["cover.jpg", "notes.txt", "album.m3u", "song.mp4"] {
            assert!(!is_audio(Path::new(name)), "{name} was read as audio");
        }
    }

    #[test]
    fn a_file_with_no_extension_is_skipped() {
        assert!(!is_audio(Path::new("README")));
        assert!(!is_audio(Path::new("song.")));
    }

    #[test]
    fn folding_does_not_stretch_past_ascii() {
        assert!(
            !is_audio(Path::new("song.mp\u{212A}")),
            "a non-ascii character was folded into a supported extension"
        );
    }
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("Library root directory not set")]
    RootNotSet,
    #[error("Could not determine the data directory")]
    DataDirNotFound,
    #[error("{0} is not an audio file the library can hold")]
    NotAudio(String),
    #[error("{0} could not be read as a track: {1}")]
    Unreadable(String, track::TrackError),
    #[error("{0} was stored but did not come back from the database")]
    NotIngested(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
