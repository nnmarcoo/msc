use rusqlite::{Connection, Result as SqliteResult};

pub fn create_tables(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            title TEXT,
            track_artist TEXT,
            album TEXT,
            album_artist TEXT,
            genre TEXT,
            year INTEGER,
            track_number INTEGER,
            disc_number INTEGER,
            comment TEXT,
            duration REAL NOT NULL,
            bit_rate INTEGER,
            sample_rate INTEGER,
            bit_depth INTEGER,
            channels INTEGER,
            missing INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS albums (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            artist TEXT,
            year INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(name, artist)
        );

        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            cover_track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER NOT NULL,
            track_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, track_id),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_tracks_path ON tracks(path);
        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(track_artist);
        CREATE INDEX IF NOT EXISTS idx_tracks_missing ON tracks(missing);
        CREATE INDEX IF NOT EXISTS idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);"
    )?;

    let _ = conn.execute(
        "ALTER TABLE playlists ADD COLUMN cover_track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL",
        [],
    );

    Ok(())
}
