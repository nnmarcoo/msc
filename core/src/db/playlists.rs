use crate::{Playlist, Track};
use rusqlite::{Result as SqliteResult, params};
use std::path::PathBuf;

use super::{Database, now};

impl Database {
    pub fn create_playlist(&self, name: &str) -> SqliteResult<i64> {
        let ts = now();
        self.conn.execute(
            "INSERT INTO playlists (name, created_at, updated_at) VALUES (?1, ?2, ?2)",
            params![name, ts],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_all_playlists(&self) -> SqliteResult<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.created_at, p.updated_at,
                    COUNT(t.id) AS track_count,
                    COALESCE(p.cover_track_id, MIN(t.id)) AS cover_track_id
             FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             LEFT JOIN tracks t ON t.id = pt.track_id AND t.missing = 0
             GROUP BY p.id
             ORDER BY p.name",
        )?;
        stmt.query_map([], |row| {
            Ok(Playlist {
                id: row.get("id")?,
                name: row.get("name")?,
                cover_track_id: row.get("cover_track_id")?,
                track_count: row.get("track_count")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
    }

    pub fn set_playlist_cover(&self, playlist_id: i64, track_id: Option<i64>) -> SqliteResult<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE playlists SET cover_track_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![track_id, ts, playlist_id],
        )?;
        Ok(())
    }

    pub fn rename_playlist(&self, id: i64, name: &str) -> SqliteResult<()> {
        let ts = now();
        self.conn.execute(
            "UPDATE playlists SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, ts, id],
        )?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> SqliteResult<()> {
        let ts = now();
        let tx = self.conn.unchecked_transaction()?;
        let position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, position],
        )?;
        tx.execute(
            "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
            params![ts, playlist_id],
        )?;
        tx.commit()
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> SqliteResult<()> {
        let ts = now();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        tx.execute(
            "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
            params![ts, playlist_id],
        )?;
        tx.commit()
    }

    pub fn get_tracks_in_playlist(&self, playlist_id: i64) -> SqliteResult<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.path, t.title, t.track_artist, t.album, t.album_artist,
                    t.genre, t.year, t.track_number, t.disc_number, t.comment,
                    t.duration, t.bit_rate, t.sample_rate, t.bit_depth, t.channels, t.missing
             FROM tracks t
             JOIN playlist_tracks pt ON pt.track_id = t.id
             WHERE pt.playlist_id = ?1 AND t.missing = 0
             ORDER BY pt.position",
        )?;
        stmt.query_map(params![playlist_id], |row| {
            Ok(Track {
                id: Some(row.get("id")?),
                path: PathBuf::from(row.get::<_, String>("path")?),
                missing: row.get::<_, i64>("missing")? != 0,
                title: row.get("title")?,
                track_artist: row.get("track_artist")?,
                album: row.get("album")?,
                album_artist: row.get("album_artist")?,
                genre: row.get("genre")?,
                year: row.get("year")?,
                track_number: row.get("track_number")?,
                disc_number: row.get("disc_number")?,
                comment: row.get("comment")?,
                duration: row.get("duration")?,
                bit_rate: row.get("bit_rate")?,
                sample_rate: row.get("sample_rate")?,
                bit_depth: row.get("bit_depth")?,
                channels: row.get("channels")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
    }
}
