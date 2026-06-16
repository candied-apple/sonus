pub mod playlists;
pub mod tracks;

use std::sync::{Arc, Mutex};

use rusqlite::{Connection, Result};

#[derive(Clone)]
pub struct Db {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn new(path: &std::path::Path) -> Self {
        let conn = Connection::open(path).expect("Failed to open SQLite database");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-8000;
             PRAGMA temp_store=MEMORY;"
        )
        .expect("Failed to set pragmas");
        Self { conn: Arc::new(Mutex::new(conn)) }
    }

    pub fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlist_tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                playlist_id INTEGER NOT NULL,
                video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                duration TEXT NOT NULL,
                duration_secs REAL NOT NULL,
                album TEXT,
                category TEXT,
                FOREIGN KEY(playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist_id ON playlist_tracks(playlist_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_playlist_tracks_video_id ON playlist_tracks(video_id)",
            [],
        )?;
        // Migrate from old "recents" table name
        let table_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='recents'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if table_count > 0 {
            conn.execute_batch(
                "ALTER TABLE recents RENAME TO history;
                 DROP INDEX IF EXISTS idx_recents_timestamp;"
            )?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                duration TEXT NOT NULL,
                duration_secs REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                album TEXT,
                category TEXT
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_history_timestamp ON history(timestamp DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_history_video_id ON history(video_id)",
            [],
        )?;
        conn.execute("DROP TABLE IF EXISTS lyrics_cache", [])?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS lyrics_cache (
                video_id TEXT PRIMARY KEY,
                lrclib_plain TEXT,
                lrclib_synced TEXT,
                ytm_plain TEXT,
                lrclib_cached_at INTEGER NOT NULL DEFAULT 0,
                ytm_cached_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
            let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let name: String = row.get(1)?;
                if name == column {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        // Run migrations for album column
        if !has_column(&conn, "playlist_tracks", "album")? {
            conn.execute("ALTER TABLE playlist_tracks ADD COLUMN album TEXT", [])?;
        }
        if !has_column(&conn, "history", "album")? {
            conn.execute("ALTER TABLE history ADD COLUMN album TEXT", [])?;
        }

        // Run migrations for category column
        if !has_column(&conn, "playlist_tracks", "category")? {
            conn.execute("ALTER TABLE playlist_tracks ADD COLUMN category TEXT DEFAULT 'Song'", [])?;
        }
        if !has_column(&conn, "history", "category")? {
            conn.execute("ALTER TABLE history ADD COLUMN category TEXT DEFAULT 'Song'", [])?;
        }

        Ok(())
    }
}
