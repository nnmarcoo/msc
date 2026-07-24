use crate::Track;
use rusqlite::{Result as SqliteResult, Row, params};
use std::path::PathBuf;

use super::Database;

const COLUMNS: &str = "id, path, title, track_artist, album, album_artist,
     genre, year, track_number, disc_number, comment,
     duration, bit_rate, sample_rate, bit_depth, channels, missing";

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
    /// The only bulk read: everything, once, at startup. Missing tracks are
    /// included — hiding them is a presentation decision made by `Library`.
    pub(crate) fn all_tracks(&self) -> SqliteResult<Vec<Track>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM tracks"))?;
        stmt.query_map([], row_to_track)?.collect()
    }

    /// One prepared statement reused across the batch. This is the only hot
    /// write path in the crate — scanning is by far the slowest operation.
    pub(crate) fn upsert_tracks(&self, tracks: &[Track]) -> SqliteResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO tracks (
                     path, title, track_artist, album, album_artist, genre,
                     year, track_number, disc_number, comment,
                     duration, bit_rate, sample_rate, bit_depth, channels, missing
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                           ?11, ?12, ?13, ?14, ?15, 0)
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
                     missing      = 0",
            )?;

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
                ])?;
            }
        }
        tx.commit()
    }

    /// Flags every track absent ahead of a scan; the upsert clears the flag for
    /// each file it finds again. Rows are never deleted — playlist membership
    /// is user-authored and cannot be re-derived, so an unplugged drive must not
    /// silently empty a playlist.
    pub(crate) fn mark_all_missing(&self) -> SqliteResult<()> {
        self.conn.execute("UPDATE tracks SET missing = 1", [])?;
        Ok(())
    }
}
