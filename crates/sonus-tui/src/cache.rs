use std::sync::Arc;
use crate::state::app_state::TrackItem;

pub(crate) fn for_you_cache_path() -> Option<std::path::PathBuf> {
    let mut path = dirs::cache_dir()?;
    path.push("sonus");
    let _ = std::fs::create_dir_all(&path);
    path.push("for_you_cache.json");
    Some(path)
}

pub(crate) fn save_for_you_cache(tracks: &[Arc<TrackItem>]) {
    let Some(path) = for_you_cache_path() else { return };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let track_refs: Vec<&TrackItem> = tracks.iter().map(|t| t.as_ref()).collect();
    let data = serde_json::json!({
        "version": 1,
        "timestamp": timestamp,
        "tracks": track_refs,
    });
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = std::fs::write(path, json);
    }
}

pub(crate) fn load_for_you_cache() -> Option<Vec<Arc<TrackItem>>> {
    let path = for_you_cache_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    let cached: serde_json::Value = serde_json::from_str(&data).ok()?;
    let timestamp = cached.get("timestamp")?.as_i64()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    if now - timestamp > 3600 {
        return None;
    }
    let tracks: Vec<TrackItem> = serde_json::from_value(cached.get("tracks")?.clone()).ok()?;
    Some(tracks.into_iter().map(Arc::new).collect())
}
