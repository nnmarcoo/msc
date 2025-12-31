use rusqlite::{Connection, Result as SqliteResult};

pub fn create_tables(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracks (
            -- Primary identification
            id INTEGER PRIMARY KEY AUTOINCREMENT,   -- Auto-incrementing ID
            path TEXT NOT NULL UNIQUE,              -- Full file path

            -- Basic metadata (from ID3/Vorbis/etc tags)
            title TEXT,                             -- Track title
            track_artist TEXT,                      -- Track artist (can differ from album artist)
            album TEXT,                             -- Album name
            album_artist TEXT,                      -- Album artist (for compilations)
            genre TEXT,                             -- Genre
            year INTEGER,                           -- Release year
            track_number INTEGER,                   -- Track number on album
            disc_number INTEGER,                    -- Disc number (for multi-disc albums)
            comment TEXT,                           -- Comment/description

            -- Audio properties (from file analysis)
            duration REAL NOT NULL,                 -- Duration in seconds (f32)
            bit_rate INTEGER,                       -- Bitrate in kbps
            sample_rate INTEGER,                    -- Sample rate in Hz (e.g., 44100, 48000)
            bit_depth INTEGER,                      -- Bit depth (e.g., 16, 24)
            channels INTEGER,                       -- Number of channels (1=mono, 2=stereo)

            -- File state
            missing INTEGER NOT NULL DEFAULT 0,     -- 1 if file not found at path, 0 if present

            -- Timestamps
            created_at INTEGER NOT NULL,            -- Unix timestamp when track was added to library
            updated_at INTEGER NOT NULL             -- Unix timestamp when track was last updated
        );

        CREATE TABLE IF NOT EXISTS albums (
            -- Primary identification
            id INTEGER PRIMARY KEY AUTOINCREMENT,   -- Auto-incrementing ID
            name TEXT NOT NULL,                     -- Album name
            artist TEXT,                            -- Album artist name
            year INTEGER,                           -- Release year

            -- Timestamps
            created_at INTEGER NOT NULL,            -- Unix timestamp when album was added to library
            updated_at INTEGER NOT NULL             -- Unix timestamp when album was last updated
        );

        CREATE TABLE IF NOT EXISTS playlists (
            -- Primary identification
            id INTEGER PRIMARY KEY AUTOINCREMENT,   -- Auto-incrementing ID
            name TEXT NOT NULL,                     -- Playlist name

            -- Timestamps
            created_at INTEGER NOT NULL,            -- Unix timestamp when playlist was created
            updated_at INTEGER NOT NULL             -- Unix timestamp when playlist was last modified
        );

        CREATE TABLE IF NOT EXISTS playlist_tracks (
            playlist_id INTEGER NOT NULL,           -- Foreign key to playlists
            track_id INTEGER NOT NULL,              -- Foreign key to tracks
            position INTEGER NOT NULL,              -- Track position in playlist (for ordering)

            PRIMARY KEY (playlist_id, track_id),
            FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
            FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_tracks_path ON tracks(path);
        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(track_artist);
        CREATE INDEX IF NOT EXISTS idx_tracks_missing ON tracks(missing);
        CREATE INDEX IF NOT EXISTS idx_playlist_tracks_position ON playlist_tracks(playlist_id, position);"
    )?;
    Ok(())
}
