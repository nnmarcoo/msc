mod albums;
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
        schema::create_tables(&conn)?;
        Ok(Database { conn })
    }
}
