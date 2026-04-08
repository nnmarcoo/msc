mod albums;
mod playlists;
mod schema;
mod tracks;

use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
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
