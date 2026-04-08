use crate::{Album, Track};
use rusqlite::{Result as SqliteResult, params};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Database;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl Database {
    pub fn batch_upsert_albums_from_tracks(&self, tracks: &[Track]) -> SqliteResult<()> {
        if tracks.is_empty() {
            return Ok(());
        }

        let mut albums: HashMap<(String, Option<String>), Option<u32>> = HashMap::new();
        for track in tracks {
            if let Some(album_name) = track.album() {
                let key = (
                    album_name.to_string(),
                    track.album_artist().map(|s| s.to_string()),
                );
                albums.entry(key).or_insert_with(|| track.year());
            }
        }

        let ts = now();

        self.conn.execute_batch("BEGIN")?;
        let result: SqliteResult<()> = (|| {
            for ((name, artist), year) in &albums {
                self.conn.execute(
                    "INSERT INTO albums (name, artist, year, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?4)
                     ON CONFLICT(name, artist) DO UPDATE SET
                         year       = excluded.year,
                         updated_at = excluded.updated_at",
                    params![name, artist, year, ts],
                )?;
            }
            Ok(())
        })();

        if result.is_ok() {
            self.conn.execute_batch("COMMIT")?;
        } else {
            self.conn.execute_batch("ROLLBACK")?;
        }
        result
    }

    pub fn get_all_albums(&self) -> SqliteResult<Vec<Album>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.artist, a.year, MIN(t.path) AS sample_track_path
             FROM albums a
             LEFT JOIN tracks t ON t.album = a.name
             GROUP BY a.id, a.name, a.artist, a.year
             ORDER BY LOWER(COALESCE(a.artist, '')), a.year NULLS LAST, LOWER(a.name)",
        )?;

        stmt.query_map([], |row| {
            Ok(Album {
                id: row.get("id")?,
                name: row.get("name")?,
                artist: row.get("artist")?,
                year: row.get("year")?,
                sample_track_path: row.get("sample_track_path")?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()
    }
}
