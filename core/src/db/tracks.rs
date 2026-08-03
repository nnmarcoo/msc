//! Track persistence.
//!
//! `rating` is user-authored and must outlive whatever the file's tags say, so
//! it appears in [`TAG_DERIVED_COLUMNS`] only on insert. A rescan refreshes the
//! tag-derived columns and leaves the rating untouched.

use crate::Track;
use rusqlite::{Result as SqliteResult, Row, params};
use std::path::PathBuf;

use super::Database;

#[cfg_attr(not(test), allow(dead_code))]
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

const SELECT_TRACKS: &str = "SELECT id, path, title, track_artist, album, album_artist,
     genre, year, track_number, disc_number, comment,
     duration, bit_rate, sample_rate, bit_depth, channels, missing, rating
     FROM tracks";

const UPSERT_TRACK: &str = "INSERT INTO tracks (
         path, title, track_artist, album, album_artist, genre,
         year, track_number, disc_number, comment,
         duration, bit_rate, sample_rate, bit_depth, channels, missing,
         rating
     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
               ?11, ?12, ?13, ?14, ?15, 0, ?16)
     ON CONFLICT(path) DO UPDATE SET
         title = excluded.title,
         track_artist = excluded.track_artist,
         album = excluded.album,
         album_artist = excluded.album_artist,
         genre = excluded.genre,
         year = excluded.year,
         track_number = excluded.track_number,
         disc_number = excluded.disc_number,
         comment = excluded.comment,
         duration = excluded.duration,
         bit_rate = excluded.bit_rate,
         sample_rate = excluded.sample_rate,
         bit_depth = excluded.bit_depth,
         channels = excluded.channels,
         missing = 0";

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
        let mut stmt = self.conn.prepare(SELECT_TRACKS)?;
        stmt.query_map([], row_to_track)?.collect()
    }

    pub(crate) fn upsert_tracks(&self, tracks: &[Track]) -> SqliteResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(UPSERT_TRACK)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The statement is written out rather than built from
    /// [`TAG_DERIVED_COLUMNS`], so this is what keeps the two in step. A column
    /// added to the list but not to the `DO UPDATE` would silently stop being
    /// refreshed by a rescan, which reads as stale tags rather than as a bug.
    #[test]
    fn every_tag_derived_column_is_refreshed_by_an_upsert() {
        for column in TAG_DERIVED_COLUMNS {
            let clause = format!("{column} = excluded.{column}");
            assert!(
                UPSERT_TRACK.contains(&clause),
                "{column} is tag-derived but a rescan would not refresh it: \
                 the upsert has no `{clause}`"
            );
        }
    }

    /// The three columns a rescan must *not* overwrite. `path` is what the
    /// conflict is detected on, `missing` is reset rather than carried from the
    /// row being inserted, and `rating` is the user's own and outlives whatever
    /// the file's tags say.
    #[test]
    fn the_columns_a_rescan_must_not_overwrite_are_left_alone() {
        for column in ["path", "rating"] {
            let clause = format!("{column} = excluded.{column}");
            assert!(
                !UPSERT_TRACK.contains(&clause),
                "a rescan would overwrite {column} with the file's own value"
            );
        }
        assert!(
            UPSERT_TRACK.contains("missing = 0"),
            "an upserted track was not marked present again"
        );
    }

    #[test]
    fn every_column_the_upsert_binds_has_a_placeholder() {
        let bound = TAG_DERIVED_COLUMNS.len() + 2;
        for n in 1..=bound {
            assert!(
                UPSERT_TRACK.contains(&format!("?{n}")),
                "the upsert binds {bound} values but has no ?{n}"
            );
        }
        assert!(
            !UPSERT_TRACK.contains(&format!("?{}", bound + 1)),
            "the upsert has more placeholders than the values bound to it"
        );
    }

    #[test]
    fn the_select_reads_every_column_a_track_is_built_from() {
        let read = [
            "id",
            "path",
            "missing",
            "rating",
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
        for column in read {
            assert!(
                SELECT_TRACKS.contains(column),
                "`row_to_track` reads {column} but the select does not fetch it"
            );
        }
    }
}
