use crate::Track;
use rusqlite::{OptionalExtension, Result as SqliteResult, Row, params};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Database;

impl Database {
    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn row_to_track(row: &Row) -> SqliteResult<Track> {
        Ok(Track::from_db(
            Some(row.get(0)?),
            PathBuf::from(row.get::<_, String>(1)?),
            row.get::<_, i64>(16)? != 0,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
            row.get(11)?,
            row.get(12)?,
            row.get(13)?,
            row.get(14)?,
            row.get(15)?,
        ))
    }

    pub fn insert_track(&self, track: &Track) -> SqliteResult<i64> {
        let now = Self::now();

        self.conn.execute(
            "INSERT INTO tracks (
                path, title, track_artist, album, album_artist, genre,
                year, track_number, disc_number, comment,
                duration, bit_rate, sample_rate, bit_depth, channels,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
            params![
                track.path().to_str(),
                track.title(),
                track.track_artist(),
                track.album(),
                track.album_artist(),
                track.genre(),
                track.year(),
                track.track_number(),
                track.disc_number(),
                track.comment(),
                track.duration(),
                track.bit_rate(),
                track.sample_rate(),
                track.bit_depth(),
                track.channels(),
                now,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn upsert_track(&self, track: &Track) -> SqliteResult<i64> {
        let now = Self::now();

        let updated = self.conn.execute(
            "UPDATE tracks SET
                title = ?1, track_artist = ?2, album = ?3, album_artist = ?4,
                genre = ?5, year = ?6, track_number = ?7, disc_number = ?8,
                comment = ?9, duration = ?10, bit_rate = ?11, sample_rate = ?12,
                bit_depth = ?13, channels = ?14, updated_at = ?15
             WHERE path = ?16",
            params![
                track.title(),
                track.track_artist(),
                track.album(),
                track.album_artist(),
                track.genre(),
                track.year(),
                track.track_number(),
                track.disc_number(),
                track.comment(),
                track.duration(),
                track.bit_rate(),
                track.sample_rate(),
                track.bit_depth(),
                track.channels(),
                now,
                track.path().to_str(),
            ],
        )?;

        if updated == 0 {
            self.insert_track(track)
        } else {
            self.conn.query_row(
                "SELECT id FROM tracks WHERE path = ?1",
                params![track.path().to_str()],
                |row| row.get(0),
            )
        }
    }

    pub fn get_track_by_id(&self, id: i64) -> SqliteResult<Option<Track>> {
        self.conn
            .query_row(
                "SELECT id, path, title, track_artist, album, album_artist,
                        genre, year, track_number, disc_number, comment,
                        duration, bit_rate, sample_rate, bit_depth, channels, missing
                 FROM tracks WHERE id = ?1",
                params![id],
                Self::row_to_track,
            )
            .optional()
    }

    pub fn get_track_by_path(&self, path: &str) -> SqliteResult<Option<Track>> {
        self.conn
            .query_row(
                "SELECT id, path, title, track_artist, album, album_artist,
                        genre, year, track_number, disc_number, comment,
                        duration, bit_rate, sample_rate, bit_depth, channels, missing
                 FROM tracks WHERE path = ?1",
                params![path],
                Self::row_to_track,
            )
            .optional()
    }

    pub fn get_all_tracks(&self) -> SqliteResult<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, track_artist, album, album_artist,
                    genre, year, track_number, disc_number, comment,
                    duration, bit_rate, sample_rate, bit_depth, channels, missing
             FROM tracks
             ORDER BY album, disc_number, track_number",
        )?;

        let tracks = stmt
            .query_map([], Self::row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(tracks)
    }

    pub fn get_n_tracks(&self, limit: i64) -> SqliteResult<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, track_artist, album, album_artist,
                    genre, year, track_number, disc_number, comment,
                    duration, bit_rate, sample_rate, bit_depth, channels, missing
             FROM tracks
             ORDER BY album, disc_number, track_number
             LIMIT ?1",
        )?;

        let tracks = stmt
            .query_map(params![limit], Self::row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(tracks)
    }

    pub fn get_tracks_by_album(&self, album_name: &str) -> SqliteResult<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, track_artist, album, album_artist,
                    genre, year, track_number, disc_number, comment,
                    duration, bit_rate, sample_rate, bit_depth, channels, missing
             FROM tracks
             WHERE album = ?1
             ORDER BY disc_number, track_number",
        )?;

        let tracks = stmt
            .query_map(params![album_name], Self::row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(tracks)
    }

    pub fn get_tracks_by_artist(&self, artist_name: &str) -> SqliteResult<Vec<Track>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, track_artist, album, album_artist,
                    genre, year, track_number, disc_number, comment,
                    duration, bit_rate, sample_rate, bit_depth, channels, missing
             FROM tracks
             WHERE track_artist = ?1 OR album_artist = ?1
             ORDER BY album, disc_number, track_number",
        )?;

        let tracks = stmt
            .query_map(params![artist_name], Self::row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(tracks)
    }

    pub fn update_track(&self, id: i64, track: &Track) -> SqliteResult<()> {
        let now = Self::now();

        self.conn.execute(
            "UPDATE tracks SET
                title = ?1, track_artist = ?2, album = ?3, album_artist = ?4,
                genre = ?5, year = ?6, track_number = ?7, disc_number = ?8,
                comment = ?9, duration = ?10, bit_rate = ?11, sample_rate = ?12,
                bit_depth = ?13, channels = ?14, updated_at = ?15
             WHERE id = ?16",
            params![
                track.title(),
                track.track_artist(),
                track.album(),
                track.album_artist(),
                track.genre(),
                track.year(),
                track.track_number(),
                track.disc_number(),
                track.comment(),
                track.duration(),
                track.bit_rate(),
                track.sample_rate(),
                track.bit_depth(),
                track.channels(),
                now,
                id,
            ],
        )?;

        Ok(())
    }

    pub fn delete_track(&self, id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_track_by_path(&self, path: &str) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM tracks WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn count_tracks(&self) -> SqliteResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
    }

    pub fn track_exists(&self, path: &str) -> SqliteResult<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE path = ?1",
            params![path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_all_missing(&self) -> SqliteResult<()> {
        self.conn.execute("UPDATE tracks SET missing = 1", [])?;
        Ok(())
    }

    pub fn mark_not_missing(&self, path: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE tracks SET missing = 0 WHERE path = ?1",
            params![path],
        )?;
        Ok(())
    }
}
