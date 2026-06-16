use rusqlite::{params, Result};

use super::Db;
use crate::types::TrackItem;

fn track_from_row(row: &rusqlite::Row) -> Result<TrackItem> {
    let video_id_str: String = row.get(0)?;
    let video_id = if video_id_str.is_empty() { None } else { Some(video_id_str) };
    let cat_str: String = row.get(6).unwrap_or_else(|_| "Song".to_string());
    let category = if cat_str == "Video" {
        crate::types::TrackCategory::Video
    } else {
        crate::types::TrackCategory::Song
    };
    Ok(TrackItem {
        index: 0,
        title: row.get(1)?,
        artist: row.get(2)?,
        duration: row.get(3)?,
        duration_secs: row.get(4)?,
        is_playing: false,
        video_id,
        album: row.get(5)?,
        category,
    })
}

impl Db {
    pub fn add_track_to_playlist(&self, playlist_id: i32, track: &TrackItem) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, video_id, title, artist, duration, duration_secs, album)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                playlist_id,
                track.video_id.as_deref().unwrap_or(""),
                track.title,
                track.artist,
                track.duration,
                track.duration_secs,
                track.album
            ],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist_id: i32) -> Result<Vec<TrackItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT video_id, title, artist, duration, duration_secs, album, category
             FROM playlist_tracks
             WHERE playlist_id = ?1
             ORDER BY id ASC"
        )?;
        let rows = stmt.query_map(params![playlist_id], |row| track_from_row(row))?;
        let mut res = Vec::new();
        for (i, r) in rows.enumerate() {
            let mut track = r?;
            track.index = i + 1;
            res.push(track);
        }
        Ok(res)
    }

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

    pub fn add_tracks_to_playlist_batch(&self, playlist_id: i32, tracks: &[TrackItem]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for track in tracks {
            let cat_str = match track.category {
                crate::types::TrackCategory::Song => "Song",
                crate::types::TrackCategory::Video => "Video",
            };
            tx.execute(
                "INSERT INTO playlist_tracks (playlist_id, video_id, title, artist, duration, duration_secs, album, category)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    playlist_id,
                    track.video_id.as_deref().unwrap_or(""),
                    track.title,
                    track.artist,
                    track.duration,
                    track.duration_secs,
                    track.album,
                    cat_str
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
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

    pub fn get_album_by_video_id(&self, video_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT album FROM history WHERE video_id = ?1 AND album != '' AND album != '-' AND album IS NOT NULL
             UNION
             SELECT album FROM playlist_tracks WHERE video_id = ?1 AND album != '' AND album != '-' AND album IS NOT NULL
             LIMIT 1"
        )?;
        let mut rows = stmt.query(params![video_id])?;
        if let Some(row) = rows.next()? {
            let album: String = row.get(0)?;
            Ok(Some(album))
        } else {
            Ok(None)
        }
    }

    pub fn update_track_album(&self, video_id: &str, album: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE history SET album = ?2 WHERE video_id = ?1",
            params![video_id, album],
        )?;
        tx.execute(
            "UPDATE playlist_tracks SET album = ?2 WHERE video_id = ?1",
            params![video_id, album],
        )?;
        tx.commit()?;
        Ok(())
    }

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

