use crate::Track;
use rusqlite::{OptionalExtension, Result as SqliteResult, Row, params};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Database;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn row_to_track(row: &Row) -> SqliteResult<Track> {
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
}

impl Database {
    pub fn batch_upsert_tracks(&self, tracks: &[Track]) -> SqliteResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }

        let ts = now();

        self.conn.execute_batch("BEGIN")?;
        let result: SqliteResult<()> = (|| {
            for track in tracks {
                self.conn.execute(
                    "INSERT INTO tracks (
                        path, title, track_artist, album, album_artist, genre,
                        year, track_number, disc_number, comment,
                        duration, bit_rate, sample_rate, bit_depth, channels,
                        created_at, updated_at, missing
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                              ?11, ?12, ?13, ?14, ?15, ?16, ?16, 0)
                    ON CONFLICT(path) DO UPDATE SET
                        title        = excluded.title,
                        track_artist = excluded.track_artist,
                        album        = excluded.album,
                        album_artist = excluded.album_artist,
                        genre        = excluded.genre,
                        year         = excluded.year,
                        track_number = excluded.track_number,
                        disc_number  = excluded.disc_number,
                        comment      = excluded.comment,
                        duration     = excluded.duration,
                        bit_rate     = excluded.bit_rate,
                        sample_rate  = excluded.sample_rate,
                        bit_depth    = excluded.bit_depth,
                        channels     = excluded.channels,
                        updated_at   = excluded.updated_at,
                        missing      = 0",
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
                        ts,
                    ],
                )?;
            }
            Ok(())
        })();

        if result.is_ok() {
            self.conn.execute_batch("COMMIT")?;
        } else {
            let _ = self.conn.execute_batch("ROLLBACK");
        }
        result
    }

    pub fn get_track_by_id(&self, id: i64) -> SqliteResult<Option<Track>> {
        self.conn
            .query_row(
                "SELECT id, path, title, track_artist, album, album_artist,
                        genre, year, track_number, disc_number, comment,
                        duration, bit_rate, sample_rate, bit_depth, channels, missing
                 FROM tracks WHERE id = ?1",
                params![id],
                row_to_track,
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
                row_to_track,
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
        stmt.query_map([], row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()
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
        stmt.query_map(params![limit], row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()
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
        stmt.query_map(params![album_name], row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()
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
        stmt.query_map(params![artist_name], row_to_track)?
            .collect::<SqliteResult<Vec<_>>>()
    }

    pub fn count_tracks(&self) -> SqliteResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
    }

    pub fn mark_all_missing(&self) -> SqliteResult<()> {
        self.conn.execute("UPDATE tracks SET missing = 1", [])?;
        Ok(())
    }
}
