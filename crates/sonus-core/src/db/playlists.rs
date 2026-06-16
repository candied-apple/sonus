use rusqlite::{params, Result};

use super::Db;

impl Db {
    pub fn create_playlist(&self, name: &str) -> Result<i32> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO playlists (name) VALUES (?1)",
            params![name],
        )?;
        let id = conn.last_insert_rowid() as i32;
        Ok(id)
    }

    pub fn delete_playlist(&self, id: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_playlists(&self) -> Result<Vec<(i32, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name FROM playlists ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i32, video_id: &str, title: &str, artist: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if !video_id.is_empty() {
            conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND video_id = ?2",
                params![playlist_id, video_id],
            )?;
        } else {
            conn.execute(
                "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND title = ?2 AND artist = ?3",
                params![playlist_id, title, artist],
            )?;
        }
        Ok(())
    }
}
