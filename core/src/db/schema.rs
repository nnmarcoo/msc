use rusqlite::{Connection, Result as SqliteResult};

/// Bumped whenever the schema changes incompatibly. On mismatch the library is
/// dropped and rebuilt from a rescan, which costs well under a second — cheaper
/// than maintaining migration paths. Playlists are carried across separately by
/// [`super::Database::open`] because they cannot be re-derived.
pub(super) const SCHEMA_VERSION: i64 = 1;

/// No `created_at`/`updated_at`: nothing ever read them.
///
/// No explicit indices either. `path UNIQUE` gives the one index the scan's
/// upsert needs, and every other read is served from memory after startup —
/// the sole remaining query is a single `SELECT *`, which is a full scan
/// regardless. Indices would only slow the scan down.
pub(super) fn create_tables(conn: &Connection) -> SqliteResult<()> {
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
            missing INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS playlists (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            cover_track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER NOT NULL,
            track_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, track_id),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
        );

        -- Playlist membership rescued from an older schema, held by path
        -- because a rescan assigns new track ids. Drained into
        -- `playlist_tracks` by `relink_pending()` once the scan has run.
        CREATE TABLE IF NOT EXISTS pending_playlist_tracks (
            playlist_id INTEGER NOT NULL,
            path TEXT NOT NULL,
            position INTEGER NOT NULL,
            is_cover INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
}

/// Everything except `playlists`/`playlist_tracks`, which the caller has
/// already read out and will reinsert once the rescan has produced fresh ids.
pub(super) fn drop_all(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS pending_playlist_tracks;
         DROP TABLE IF EXISTS playlist_tracks;
         DROP TABLE IF EXISTS playlists;
         DROP TABLE IF EXISTS albums;
         DROP TABLE IF EXISTS tracks;
         DROP TABLE IF EXISTS meta;",
    )
}
