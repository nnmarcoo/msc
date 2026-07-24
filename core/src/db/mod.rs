mod playlists;
mod schema;
mod tracks;

use rusqlite::{Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;

use schema::SCHEMA_VERSION;

/// A playlist rescued across a schema rebuild.
///
/// Track ids are not stable across a rescan, so membership is carried by path
/// and re-linked once the new track rows exist.
struct RescuedPlaylist {
    name: String,
    cover_path: Option<String>,
    track_paths: Vec<String>,
}

pub(crate) struct Database {
    conn: Connection,
}

impl Database {
    pub(crate) fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;",
        )?;

        let db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Rebuilds the library on schema mismatch rather than migrating it.
    ///
    /// A full rescan costs a few hundred milliseconds, so version-by-version
    /// `ALTER TABLE` paths are not worth their maintenance. Playlists are the
    /// exception — they are user-authored and unrecoverable, so they are read
    /// out before the drop and reinserted afterwards, re-linked by path.
    fn migrate(&self) -> SqliteResult<()> {
        let version = self.schema_version()?;
        if version == Some(SCHEMA_VERSION) {
            return Ok(());
        }

        let rescued = match version {
            // A pre-versioning database. Salvage playlists, discard the rest.
            None if self.table_exists("playlists")? => self.rescue_playlists()?,
            _ => Vec::new(),
        };

        schema::drop_all(&self.conn)?;
        schema::create_tables(&self.conn)?;
        self.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;

        for playlist in &rescued {
            self.restore_playlist(playlist)?;
        }
        Ok(())
    }

    fn schema_version(&self) -> SqliteResult<Option<i64>> {
        if !self.table_exists("meta")? {
            return Ok(None);
        }
        Ok(self
            .get_meta("schema_version")?
            .and_then(|v| v.parse().ok()))
    }

    fn table_exists(&self, name: &str) -> SqliteResult<bool> {
        self.conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |_| Ok(()),
            )
            .optional()
            .map(|row| row.is_some())
    }

    pub(crate) fn get_meta(&self, key: &str) -> SqliteResult<Option<String>> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
    }

    pub(crate) fn set_meta(&self, key: &str, value: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub(crate) fn delete_meta(&self, key: &str) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM meta WHERE key = ?1", [key])?;
        Ok(())
    }

    pub(crate) fn clear_library(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "DELETE FROM playlist_tracks;
             DELETE FROM playlists;
             DELETE FROM tracks;",
        )?;
        self.delete_meta("root")
    }
}

impl Drop for Database {
    /// Without this the WAL grows unboundedly across runs — the connection was
    /// previously never closed cleanly, leaving a 4 KB database beside a 638 KB
    /// write-ahead log.
    fn drop(&mut self) {
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
}

/// Playlist rescue across a schema rebuild.
impl Database {
    fn rescue_playlists(&self) -> SqliteResult<Vec<RescuedPlaylist>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM playlists")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<SqliteResult<_>>()?;

        rows.into_iter()
            .map(|(id, name)| {
                Ok(RescuedPlaylist {
                    name,
                    cover_path: self.rescued_cover_path(id)?,
                    track_paths: self.rescued_track_paths(id)?,
                })
            })
            .collect()
    }

    fn rescued_cover_path(&self, playlist_id: i64) -> SqliteResult<Option<String>> {
        self.conn
            .query_row(
                "SELECT t.path FROM tracks t
                 JOIN playlists p ON p.cover_track_id = t.id
                 WHERE p.id = ?1",
                [playlist_id],
                |row| row.get(0),
            )
            .optional()
    }

    fn rescued_track_paths(&self, playlist_id: i64) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.path FROM tracks t
             JOIN playlist_tracks pt ON pt.track_id = t.id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )?;
        stmt.query_map([playlist_id], |row| row.get(0))?.collect()
    }

    /// Recreates the playlist and stages its membership by path.
    ///
    /// The referenced tracks do not exist yet — the rebuild dropped them and the
    /// next scan recreates them with fresh ids — so membership is parked in
    /// `pending_playlist_tracks` and resolved later by [`Self::relink_pending`].
    fn restore_playlist(&self, playlist: &RescuedPlaylist) -> SqliteResult<()> {
        let id = self.create_playlist(&playlist.name)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO pending_playlist_tracks (playlist_id, path, position, is_cover)
             VALUES (?1, ?2, ?3, ?4)",
        )?;

        for (position, path) in playlist.track_paths.iter().enumerate() {
            stmt.execute(rusqlite::params![id, path, position as i64, 0])?;
        }
        if let Some(cover) = &playlist.cover_path {
            stmt.execute(rusqlite::params![id, cover, -1_i64, 1])?;
        }
        Ok(())
    }

    /// Resolves staged membership against the track rows a scan has just
    /// produced. Paths the scan did not find stay pending, so a playlist whose
    /// files are temporarily unavailable is not quietly emptied.
    pub(crate) fn relink_pending(&self) -> SqliteResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             SELECT p.playlist_id, t.id, p.position
             FROM pending_playlist_tracks p
             JOIN tracks t ON t.path = p.path
             WHERE p.is_cover = 0",
            [],
        )?;

        tx.execute(
            "UPDATE playlists SET cover_track_id = (
                 SELECT t.id FROM pending_playlist_tracks p
                 JOIN tracks t ON t.path = p.path
                 WHERE p.is_cover = 1 AND p.playlist_id = playlists.id
             )
             WHERE cover_track_id IS NULL
               AND EXISTS (
                 SELECT 1 FROM pending_playlist_tracks p
                 JOIN tracks t ON t.path = p.path
                 WHERE p.is_cover = 1 AND p.playlist_id = playlists.id
               )",
            [],
        )?;

        tx.execute(
            "DELETE FROM pending_playlist_tracks
             WHERE path IN (SELECT path FROM tracks)",
            [],
        )?;

        tx.commit()
    }
}
