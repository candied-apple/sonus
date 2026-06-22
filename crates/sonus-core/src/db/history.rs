use rusqlite::{params, Result};
use super::Db;
use crate::types::TrackItem;
use super::tracks::track_from_row;

impl Db {
    pub fn add_history_track(&self, track: &TrackItem, limit: usize) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        
        let cat_str = match track.category {
            crate::types::TrackCategory::Song => "Song",
            crate::types::TrackCategory::Video => "Video",
        };

        conn.execute(
            "INSERT INTO history (video_id, title, artist, duration, duration_secs, timestamp, album, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                track.video_id.as_deref().unwrap_or(""),
                track.title,
                track.artist,
                track.duration,
                track.duration_secs,
                timestamp,
                track.album,
                cat_str
            ],
        )?;

        // Enforce the history limit in database
        conn.execute(
            "DELETE FROM history WHERE id NOT IN (
                SELECT id FROM history ORDER BY timestamp DESC LIMIT ?1
            )",
            params![limit],
        )?;

        Ok(())
    }

    pub fn get_history_tracks(&self) -> Result<Vec<TrackItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration, duration_secs, album, category
             FROM history
             ORDER BY timestamp DESC"
        )?;
        let rows = stmt.query_map([], |row| track_from_row(row))?;
        let mut res = Vec::new();
        for (i, r) in rows.enumerate() {
            let mut track = r?;
            track.index = i + 1;
            res.push(track);
        }
        Ok(res)
    }

    pub fn clear_history_tracks(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn get_recent_unique_tracks(&self, limit: usize) -> Result<Vec<TrackItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration, duration_secs, album, category, MAX(timestamp) as max_ts
             FROM history
             WHERE video_id != ''
             GROUP BY video_id
             ORDER BY max_ts DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| track_from_row(row))?;
        let mut res = Vec::new();
        for (i, r) in rows.enumerate() {
            let mut track = r?;
            track.index = i + 1;
            res.push(track);
        }
        Ok(res)
    }

    pub fn get_top_artists(&self, limit: usize) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as play_count
             FROM history
             WHERE artist != '' AND (category = 'Song' OR category IS NULL OR category = '')
             GROUP BY artist
             ORDER BY play_count DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let artist: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((artist, count as usize))
        })?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn get_top_channels(&self, limit: usize) -> Result<Vec<(String, usize)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT artist, COUNT(*) as play_count
             FROM history
             WHERE artist != '' AND category = 'Video'
             GROUP BY artist
             ORDER BY play_count DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let channel: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((channel, count as usize))
        })?;
        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn get_seed_tracks_by_category(&self, category: crate::types::TrackCategory, limit: usize) -> Result<Vec<TrackItem>> {
        let conn = self.conn.lock().unwrap();
        let cat_str = match category {
            crate::types::TrackCategory::Song => "Song",
            crate::types::TrackCategory::Video => "Video",
        };
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration, duration_secs, album, category, COUNT(*) as plays, MAX(timestamp) as last_ts
             FROM history
             WHERE video_id != '' AND category = ?2
             GROUP BY video_id
             ORDER BY (COUNT(*) * 1.0 / (1.0 + (CAST(strftime('%s', 'now') AS INTEGER) - MAX(timestamp)) / 86400.0)) DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit, cat_str], |row| track_from_row(row))?;
        let mut res = Vec::new();
        for (i, r) in rows.enumerate() {
            let mut track = r?;
            track.index = i + 1;
            res.push(track);
        }
        Ok(res)
    }

    pub fn get_seed_tracks(&self, limit: usize) -> Result<Vec<TrackItem>> {
        let conn = self.conn.lock().unwrap();
        // Return most played/recent tracks regardless of category
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration, duration_secs, album, category, COUNT(*) as plays, MAX(timestamp) as last_ts
             FROM history
             WHERE video_id != ''
             GROUP BY video_id
             ORDER BY
               (COUNT(*) * 1.0 / (1.0 + (CAST(strftime('%s', 'now') AS INTEGER) - MAX(timestamp)) / 86400.0)) DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| track_from_row(row))?;
        let mut res = Vec::new();
        for (i, r) in rows.enumerate() {
            let mut track = r?;
            track.index = i + 1;
            res.push(track);
        }
        Ok(res)
    }

    pub fn delete_history_track(&self, video_id: &str, title: &str, artist: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if !video_id.is_empty() {
            conn.execute(
                "DELETE FROM history WHERE video_id = ?1",
                params![video_id],
            )?;
        } else {
            conn.execute(
                "DELETE FROM history WHERE title = ?1 AND artist = ?2",
                params![title, artist],
            )?;
        }
        Ok(())
    }
}
