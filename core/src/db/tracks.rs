//! Track persistence.
//!
//! `rating` is user-authored and must outlive whatever the file's tags say, so
//! it appears in [`TAG_DERIVED_COLUMNS`] only on insert. A rescan refreshes the
//! tag-derived columns and leaves the rating untouched.

use crate::Track;
use rusqlite::{Result as SqliteResult, Row, params};
use std::path::PathBuf;

use super::Database;

const COLUMNS: &str = "id, path, title, track_artist, album, album_artist,
     genre, year, track_number, disc_number, comment,
     duration, bit_rate, sample_rate, bit_depth, channels, missing, rating";

const TAG_DERIVED_COLUMNS: &[&str] = &[
    "title",
    "track_artist",
    "album",
    "album_artist",
    "genre",
    "year",
    "track_number",
    "disc_number",
    "comment",
    "duration",
    "bit_rate",
    "sample_rate",
    "bit_depth",
    "channels",
];

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
        rating: row.get::<_, Option<i64>>("rating")?.map(|r| r as u8),
    })
}

impl Database {
    pub(crate) fn all_tracks(&self) -> SqliteResult<Vec<Track>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM tracks"))?;
        stmt.query_map([], row_to_track)?.collect()
    }

    pub(crate) fn upsert_tracks(&self, tracks: &[Track]) -> SqliteResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }

        let refreshed = TAG_DERIVED_COLUMNS
            .iter()
            .map(|column| format!("{column} = excluded.{column}"))
            .collect::<Vec<_>>()
            .join(", ");

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(&format!(
                "INSERT INTO tracks (
                     path, title, track_artist, album, album_artist, genre,
                     year, track_number, disc_number, comment,
                     duration, bit_rate, sample_rate, bit_depth, channels, missing,
                     rating
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                           ?11, ?12, ?13, ?14, ?15, 0, ?16)
                 ON CONFLICT(path) DO UPDATE SET {refreshed}, missing = 0"
            ))?;

            for track in tracks {
                stmt.execute(params![
                    track.path.to_str(),
                    track.title,
                    track.track_artist,
                    track.album,
                    track.album_artist,
                    track.genre,
                    track.year,
                    track.track_number,
                    track.disc_number,
                    track.comment,
                    track.duration,
                    track.bit_rate,
                    track.sample_rate,
                    track.bit_depth,
                    track.channels,
                    track.rating,
                ])?;
            }
        }
        tx.commit()
    }

    pub(crate) fn mark_all_missing(&self) -> SqliteResult<()> {
        self.conn.execute("UPDATE tracks SET missing = 1", [])?;
        Ok(())
    }

    pub(crate) fn set_rating(&self, track_id: i64, rating: Option<u8>) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE tracks SET rating = ?1 WHERE id = ?2",
            params![rating.map(i64::from), track_id],
        )?;
        Ok(())
    }
}
