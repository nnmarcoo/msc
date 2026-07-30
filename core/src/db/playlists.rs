use rusqlite::{Result as SqliteResult, params};

use super::Database;

impl Database {
    pub(crate) fn all_playlists(&self) -> SqliteResult<Vec<(i64, String, Option<i64>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, cover_track_id FROM playlists ORDER BY name")?;
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect()
    }

    pub(crate) fn all_playlist_tracks(&self) -> SqliteResult<Vec<(i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT playlist_id, track_id FROM playlist_tracks
             ORDER BY playlist_id, position",
        )?;
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect()
    }

    pub(crate) fn create_playlist(&self, name: &str) -> SqliteResult<i64> {
        self.conn
            .execute("INSERT INTO playlists (name) VALUES (?1)", params![name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub(crate) fn rename_playlist(&self, id: i64, name: &str) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE playlists SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub(crate) fn delete_playlist(&self, id: i64) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub(crate) fn set_playlist_cover(
        &self,
        playlist_id: i64,
        track_id: Option<i64>,
    ) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE playlists SET cover_track_id = ?1 WHERE id = ?2",
            params![track_id, playlist_id],
        )?;
        Ok(())
    }

    pub(crate) fn add_track_to_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
        position: i64,
    ) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, position],
        )?;
        Ok(())
    }

    pub(crate) fn remove_track_from_playlist(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }
}
