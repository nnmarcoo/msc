mod schema;
mod tracks;

use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: Option<&Path>) -> SqliteResult<Self> {
        let conn = match db_path {
            Some(path) => Connection::open(path)?,
            None => Connection::open_in_memory()?,
        };
        schema::create_tables(&conn)?;
        Ok(Database { conn })
    }
}
