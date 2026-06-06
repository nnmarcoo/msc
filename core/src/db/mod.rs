mod albums;
mod playlists;
mod schema;
mod tracks;

use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )?;
        schema::create_tables(&conn)?;
        Ok(Database { conn })
    }

    pub fn clear_library(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "DELETE FROM playlist_tracks;
             DELETE FROM playlists;
             DELETE FROM albums;
             DELETE FROM tracks;",
        )
    }
}
