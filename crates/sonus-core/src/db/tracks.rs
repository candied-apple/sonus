use rusqlite::{params, Result};

use super::Db;
use crate::types::TrackItem;

pub(crate) fn track_from_row(row: &rusqlite::Row) -> Result<TrackItem> {
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
}
