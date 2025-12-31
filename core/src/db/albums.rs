use crate::Track;
use rusqlite::{Result as SqliteResult, params};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Database;

impl Database {
    fn now_albums() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    pub fn upsert_album(
        &self,
        name: &str,
        artist: Option<&str>,
        year: Option<u32>,
    ) -> SqliteResult<i64> {
        let now = Self::now_albums();

        let updated = self.conn.execute(
            "UPDATE albums SET
                artist = ?1, year = ?2, updated_at = ?3
             WHERE name = ?4",
            params![artist, year, now, name],
        )?;

        if updated == 0 {
            self.conn.execute(
                "INSERT INTO albums (name, artist, year, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![name, artist, year, now],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.query_row(
                "SELECT id FROM albums WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
        }
    }

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

        let now = Self::now_albums();

        self.conn.execute_batch("BEGIN TRANSACTION")?;

        let result = (|| {
            for ((album_name, artist), year) in albums.iter() {
                let updated = self.conn.execute(
                    "UPDATE albums SET
                        year = ?1, updated_at = ?2
                     WHERE name = ?3 AND artist IS ?4",
                    params![year, now, album_name, artist],
                )?;

                if updated == 0 {
                    self.conn.execute(
                        "INSERT INTO albums (name, artist, year, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?4)",
                        params![album_name, artist, year, now],
                    )?;
                }
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

    pub fn get_all_albums(
        &self,
    ) -> SqliteResult<Vec<(i64, String, Option<String>, Option<u32>, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.artist, a.year,
                    (SELECT path FROM tracks t WHERE t.album = a.name LIMIT 1) as sample_track_path
             FROM albums a
             ORDER BY a.name",
        )?;

        let albums = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(albums)
    }

    pub fn delete_album(&self, id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM albums WHERE id = ?1", params![id])?;
        Ok(())
    }
}
