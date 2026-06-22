use std::sync::{Arc, Mutex};
use rusqlite::Connection;

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
}
