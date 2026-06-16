use std::sync::Arc;
use tokio::sync::oneshot;
use sonus_core::api::client::YtmClient;
use crate::app::{ApiResult, App};
use crate::state::app_state::Focus;

impl App {
    pub fn load_explore_data(&mut self, ytm: &Arc<YtmClient>) {
        // Query Top Artists from DB
        match self.db.get_top_artists(10) {
            Ok(artists) => {
                self.state.explore_top_artists = artists;
            }
            Err(e) => {
                self.state.status_message = Some(format!("DB Error (artists): {}", e));
            }
        }

        // Query Top Channels from DB
        match self.db.get_top_channels(10) {
            Ok(channels) => {
                self.state.explore_top_channels = channels;
            }
            Err(e) => {
                self.state.status_message = Some(format!("DB Error (channels): {}", e));
            }
        }

        if self.state.explore_loaded && !self.state.explore_for_you.is_empty() {
            return;
        }

        // Try loading from disk cache first
        if let Some(cached) = crate::app::load_for_you_cache() {
            self.state.explore_for_you = cached;
            self.state.explore_loaded = true;
            return;
        }

        // Fetch recommendations
        self.refresh_for_you(ytm);
        self.state.explore_loaded = true;
    }

    pub fn refresh_for_you(&mut self, ytm: &Arc<YtmClient>) {
        let song_seeds = self.db.get_seed_tracks_by_category(sonus_core::types::TrackCategory::Song, 3).unwrap_or_default();
        let video_seeds = self.db.get_seed_tracks_by_category(sonus_core::types::TrackCategory::Video, 2).unwrap_or_default();
        
        let mut seeds = song_seeds;
        seeds.extend(video_seeds);

        if seeds.is_empty() {
            // Fallback to general seeds if no categorized history is found (e.g. after migration)
            if let Ok(general_seeds) = self.db.get_seed_tracks(5) {
                seeds = general_seeds;
            }
        }
        
        // If we still don't have enough variety, look for things that look like videos
        // (no album and not from a topic channel) even if they are marked as songs.
        if seeds.len() < 5 {
             if let Ok(history) = self.db.get_history_tracks() {
                 let mut additional = history.into_iter()
                    .filter(|t| t.category == sonus_core::types::TrackCategory::Song)
                    .filter(|t| t.album.is_none() || t.album.as_deref() == Some("-"))
                    .filter(|t| !t.artist.to_lowercase().contains("- topic"))
                    .take(5 - seeds.len())
                    .collect::<Vec<_>>();
                 seeds.append(&mut additional);
             }
        }

        if seeds.is_empty() {
            self.state.explore_for_you.clear();
            self.state.status_message = Some("No history found to generate recommendations".into());
            return;
        }

        let ytm = Arc::clone(ytm);
        let (tx, rx) = oneshot::channel();
        self.pending_api = Some(rx);
        self.state.status_message = Some("Loading personalized recommendations...".into());

        let db = self.db.clone();
        tokio::spawn(async move {
            let mut all_tracks = Vec::new();
            
            // Limit to 3 seeds to be faster and less likely to be blocked
            let seeds_to_use = if seeds.len() > 3 { &seeds[0..3] } else { &seeds };
            
            for track in seeds_to_use {
                if let Some(video_id) = track.video_id.as_ref() {
                    match ytm.get_recommendations_categorized(video_id).await {
                        Ok(tracks) => {
                            for mut t in tracks {
                                if let Some(ref vid) = t.video_id {
                                    if let Ok(Some(album_name)) = db.get_album_by_video_id(vid) {
                                        t.album = Some(album_name);
                                    }
                                }
                                all_tracks.push(t);
                            }
                        }
                        Err(e) => {
                            sonus_core::log!("Failed to get recommendations for {}: {}", track.title, e);
                        }
                    }
                    // Small delay to avoid triggering rate limits
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }

            let _ = tx.send(ApiResult::ExploreRecommendations(all_tracks));
        });
    }

    pub fn select_artist(&mut self, artist_name: &str, ytm: &Arc<YtmClient>) {
        let (tx, rx) = oneshot::channel();
        self.pending_api = Some(rx);
        self.state.status_message = Some(format!("Searching songs for {}...", artist_name));
        self.state.active_page = crate::state::app_state::ActivePage::Search;
        self.state.focus = Focus::Tracklist;
        self.state.search_query = artist_name.to_string();
        self.state.view_title = format!("Artist: {}", artist_name);
        self.state.song_index = 0;
        self.state.song_offset = 0;
        self.state.video_index = 0;
        self.state.video_offset = 0;

        let ytm = Arc::clone(ytm);
        let q = artist_name.to_string();
        tokio::spawn(async move {
            match ytm.search_all(&q).await {
                Ok(tracks) => { let _ = tx.send(ApiResult::Tracks(tracks)); }
                Err(e) => { let _ = tx.send(ApiResult::Error(e)); }
            }
        });
    }
}
