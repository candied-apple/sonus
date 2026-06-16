use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::app::App;
use crate::state::app_state::TrackItem;

#[derive(Debug)]
pub(crate) enum ImportProgress {
    Started { playlist_name: String, total: usize },
    TrackProcessing { title: String, artist: String },
    TrackResolved { completed: usize, total: usize },
    Success { playlist_name: String, count: usize },
    Error(String),
}

impl App {
    pub(crate) fn start_spotify_import(&mut self, url_or_id: String) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        self.import_rx = Some(rx);

        self.state.spotify_import = Some(crate::state::app_state::SpotifyImportState {
            playlist_name: "Spotify Playlist".to_string(),
            completed: 0,
            total_tracks: 0,
            current_track_name: "Initializing & fetching playlist tracks...".to_string(),
        });

        let ytm = match &self.ytm {
            Some(y) => Arc::clone(y),
            None => {
                let _ = tx.try_send(ImportProgress::Error("YTM client not ready".to_string()));
                return;
            }
        };
        let db = self.db.clone();

        tokio::spawn(async move {
            let mut playlist = spotapi::PublicPlaylist::new(&url_or_id);
            
            let raw_tracks = match playlist.get_tracks().await {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(ImportProgress::Error(format!("Failed to fetch Spotify playlist: {}", e))).await;
                    return;
                }
            };

            let mut spotify_tracks = Vec::new();
            for item in raw_tracks {
                if let Some(data) = item.get("itemV2").and_then(|i| i.get("data")) {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown").to_string();
                    let artist = data.get("artists")
                        .and_then(|a| a.get("items"))
                        .and_then(|i| i.as_array())
                        .and_then(|a| a.first())
                        .and_then(|a| a.get("profile"))
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let duration_ms = data.get("trackDuration")
                        .and_then(|d| d.get("totalMilliseconds"))
                        .and_then(|m| m.as_u64())
                        .unwrap_or(0);
                    spotify_tracks.push((name, artist, duration_ms as f64 / 1000.0));
                }
            }

            if spotify_tracks.is_empty() {
                let _ = tx.send(ImportProgress::Error("No tracks found in playlist".to_string())).await;
                return;
            }

            let playlist_name = match playlist.get_playlist_info(1, 0).await {
                Ok(info) => {
                    info.get("data")
                        .and_then(|d| d.get("playlistV2"))
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Spotify Import")
                        .to_string()
                }
                Err(_) => "Spotify Import".to_string(),
            };

            let mut final_name = playlist_name.clone();
            let existing: Vec<_> = db.get_playlists().unwrap_or_default();
            let mut idx = 1;
            while existing.iter().any(|(_, name)| name == &final_name) {
                final_name = format!("{} ({})", playlist_name, idx);
                idx += 1;
            }

            let playlist_id = match db.create_playlist(&final_name) {
                Ok(id) => id,
                Err(e) => {
                    let _ = tx.send(ImportProgress::Error(format!("Failed to create playlist in DB: {}", e))).await;
                    return;
                }
            };

            let total = spotify_tracks.len();
            let _ = tx.send(ImportProgress::Started { playlist_name: final_name.clone(), total }).await;

            let resolved = Arc::new(std::sync::Mutex::new(vec![None; total]));
            let completed_counter = Arc::new(AtomicUsize::new(0));
            let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
            let mut handles = Vec::new();

            for (index, (title, artist, duration_secs)) in spotify_tracks.into_iter().enumerate() {
                let sem = semaphore.clone();
                let ytm_c = Arc::clone(&ytm);
                let tx = tx.clone();
                let resolved = Arc::clone(&resolved);
                let completed_counter = Arc::clone(&completed_counter);

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire_owned().await;
                    let _ = tx.send(ImportProgress::TrackProcessing {
                        title: title.clone(),
                        artist: artist.clone(),
                    }).await;

                    let query = format!("{} {}", title, artist);
                    match ytm_c.search_songs(&query).await {
                        Ok(results) => {
                            let matched_track = match_track(&title, &artist, duration_secs, &results);
                            if let Some(track) = matched_track {
                                resolved.lock().unwrap()[index] = Some(track);
                            }
                        }
                        Err(e) => {
                            sonus_core::log!("Search failed for {}: {}", query, e);
                        }
                    }
                    completed_counter.fetch_add(1, Ordering::Relaxed);
                    let _ = tx.send(ImportProgress::TrackResolved {
                        completed: completed_counter.load(Ordering::Relaxed),
                        total,
                    }).await;
                }));
            }

            for handle in handles {
                let _ = handle.await;
            }

            let resolved_tracks: Vec<TrackItem> = resolved
                .lock()
                .unwrap()
                .iter()
                .filter_map(|track| track.clone())
                .collect();
            let import_count = resolved_tracks.len();
            if !resolved_tracks.is_empty()
                && let Err(e) = db.add_tracks_to_playlist_batch(playlist_id, &resolved_tracks)
            {
                sonus_core::log!("Failed to batch insert tracks: {}", e);
            }

            let _ = tx.send(ImportProgress::Success { playlist_name: final_name, count: import_count }).await;
        });
    }
}

fn normalize_str(s: &str) -> String {
    let mut normalized = s.to_lowercase();
    let suffixes = [
        "(official video)",
        "(official audio)",
        "(lyric video)",
        "(lyrics)",
        " - official video",
        " - official audio",
        " - remastered",
        "(remastered)",
    ];
    for suffix in suffixes {
        if let Some(idx) = normalized.find(suffix) {
            normalized.truncate(idx);
        }
    }
    normalized
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn match_track(
    spotify_title: &str,
    spotify_artist: &str,
    spotify_duration_secs: f64,
    ytm_results: &[TrackItem],
) -> Option<TrackItem> {
    if ytm_results.is_empty() {
        return None;
    }

    let norm_spotify_title = normalize_str(spotify_title);
    let norm_spotify_artist = normalize_str(spotify_artist);

    let mut best_match: Option<(TrackItem, f64)> = None;

    for result in ytm_results.iter().take(3) {
        let mut score = 0.0;

        let dur_diff = (result.duration_secs - spotify_duration_secs).abs();
        if dur_diff <= 5.0 {
            score += 100.0;
        } else if dur_diff <= 10.0 {
            score += 50.0;
        } else if dur_diff > 20.0 {
            score -= 100.0;
        }

        let norm_ytm_title = normalize_str(&result.title);
        if norm_ytm_title == norm_spotify_title {
            score += 100.0;
        } else if norm_ytm_title.contains(&norm_spotify_title) || norm_spotify_title.contains(&norm_ytm_title) {
            score += 50.0;
        }

        let norm_ytm_artist = normalize_str(&result.artist);
        if norm_ytm_artist == norm_spotify_artist {
            score += 50.0;
        } else if norm_ytm_artist.contains(&norm_spotify_artist) || norm_spotify_artist.contains(&norm_ytm_artist) {
            score += 25.0;
        }

        if score > 0.0 {
            if let Some((_, best_score)) = best_match {
                if score > best_score {
                    best_match = Some((result.clone(), score));
                }
            } else {
                best_match = Some((result.clone(), score));
            }
        }
    }

    best_match.map(|(track, _)| track).or_else(|| ytm_results.first().cloned())
}
