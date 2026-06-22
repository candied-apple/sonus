use rusqlite::{params, Result};
use super::Db;

impl Db {
    pub fn get_cached_lyrics(&self, video_id: &str) -> Result<Option<(Option<String>, Option<String>, Option<String>, i64, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT lrclib_plain, lrclib_synced, ytm_plain, lrclib_cached_at, ytm_cached_at 
             FROM lyrics_cache WHERE video_id = ?1"
        )?;
        let mut rows = stmt.query(params![video_id])?;
        if let Some(row) = rows.next()? {
            let lrclib_plain: Option<String> = row.get(0)?;
            let lrclib_synced: Option<String> = row.get(1)?;
            let ytm_plain: Option<String> = row.get(2)?;
            let lrclib_cached_at: i64 = row.get(3)?;
            let ytm_cached_at: i64 = row.get(4)?;
            Ok(Some((lrclib_plain, lrclib_synced, ytm_plain, lrclib_cached_at, ytm_cached_at)))
        } else {
            Ok(None)
        }
    }

    pub fn cache_lrclib_lyrics(&self, video_id: &str, plain: Option<&str>, synced: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        conn.execute(
            "INSERT INTO lyrics_cache (video_id, lrclib_plain, lrclib_synced, lrclib_cached_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(video_id) DO UPDATE SET
             lrclib_plain = excluded.lrclib_plain,
             lrclib_synced = excluded.lrclib_synced,
             lrclib_cached_at = excluded.lrclib_cached_at",
            params![video_id, plain, synced, timestamp],
        )?;
        Ok(())
    }

    pub fn cache_ytm_lyrics(&self, video_id: &str, plain: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        conn.execute(
            "INSERT INTO lyrics_cache (video_id, ytm_plain, ytm_cached_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(video_id) DO UPDATE SET
             ytm_plain = excluded.ytm_plain,
             ytm_cached_at = excluded.ytm_cached_at",
            params![video_id, plain, timestamp],
        )?;
        Ok(())
    }
}
