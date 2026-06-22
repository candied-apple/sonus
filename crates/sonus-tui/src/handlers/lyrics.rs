use std::sync::Arc;
use tokio::sync::oneshot;
use crate::app::{ApiResult, App};
use crate::lrc::parse_lrc;

impl App {
    pub(crate) fn fetch_lyrics_for_current_track(&mut self) {
        let ytm = match &self.ytm {
            Some(y) => Arc::clone(y),
            None => return,
        };

        let current_track = self.state.player.current_track.clone();
        let duration = self.state.player.duration;

        if let Some(video_id) = &self.state.player.current_video_id {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0) as i64;

            let mut lrclib_plain = None;
            let mut lrclib_synced = None;
            let mut ytm_plain = None;
            let mut lrclib_cached_at = 0i64;
            let mut ytm_cached_at = 0i64;

            if let Ok(Some((lp, ls, yp, lca, yca))) = self.db.get_cached_lyrics(video_id) {
                lrclib_plain = lp;
                lrclib_synced = ls;
                ytm_plain = yp;
                lrclib_cached_at = lca;
                ytm_cached_at = yca;
            }

            // Scenario A: We have synced lyrics cached. Display them instantly and return (no API).
            if lrclib_synced.is_some() {
                self.state.lyrics_text = lrclib_plain.or_else(|| Some("Synced lyrics loaded".into()));
                self.state.synced_lyrics = lrclib_synced.map(|s| parse_lrc(&s));
                return;
            }

            // Scenario B: We don't have synced lyrics cached.
            // If we have plain lyrics (either LRCLib or YTM), display them instantly.
            let displayed_plain = lrclib_plain.or(ytm_plain);
            if displayed_plain.is_some() {
                self.state.lyrics_text = displayed_plain;
                self.state.synced_lyrics = None;
            }

            // Check if we should query the APIs in the background.
            // We query LRCLib if we haven't tried in the last 3 days (259,200 seconds).
            // We query YTM if we don't have ytm_plain cached and haven't tried YTM.
            let should_query_lrclib = now - lrclib_cached_at >= 259200;
            let should_query_ytm = ytm_cached_at == 0;

            if (should_query_lrclib || should_query_ytm) && self.pending_api.is_none() {
                let video_id = video_id.clone();
                let (title, artist) = current_track.unwrap_or_else(|| ("".into(), "".into()));
                let (tx, rx) = oneshot::channel();
                self.pending_api = Some(rx);

                if self.state.lyrics_text.is_none() {
                    self.state.lyrics_text = Some("Loading lyrics...".into());
                    self.state.synced_lyrics = None;
                }

                let db_clone = self.db.clone();
                let ytm_clone = Arc::clone(&ytm);

                tokio::spawn(async move {
                    let mut plain = None;
                    let mut synced = None;
                    let mut lrclib_success = false;

                    if should_query_lrclib {
                        if let Ok((p, s)) = ytm_clone.get_lyric_from_lrclib(&artist, &title, duration).await {
                            if p.is_some() || s.is_some() {
                                plain = p;
                                synced = s;
                                lrclib_success = true;
                                let _ = db_clone.cache_lrclib_lyrics(&video_id, plain.as_deref(), synced.as_deref());
                            }
                        }
                        if !lrclib_success {
                            // Mark LRCLib as tried, even though it returned nothing
                            let _ = db_clone.cache_lrclib_lyrics(&video_id, None, None);
                        }
                    }

                    if synced.is_some() {
                        let _ = tx.send(ApiResult::Lyrics { plain, synced });
                        return;
                    }

                    // Try YTM if needed
                    let mut final_plain = plain;
                    if should_query_ytm {
                        if let Ok(y_plain) = ytm_clone.get_ytm_lyrics(&video_id).await {
                            let _ = db_clone.cache_ytm_lyrics(&video_id, Some(&y_plain));
                            final_plain = final_plain.or(Some(y_plain.to_string()));
                        }
                    } else {
                        // Use existing cached YTM lyrics if available
                        if let Ok(Some((_, _, yp, _, _))) = db_clone.get_cached_lyrics(&video_id) {
                            final_plain = final_plain.or(yp);
                        }
                    }

                    if final_plain.is_some() {
                        let _ = tx.send(ApiResult::Lyrics { plain: final_plain, synced: None });
                    } else {
                        let _ = tx.send(ApiResult::Error("No lyrics available".to_string()));
                    }
                });
            }
        } else {
            self.state.lyrics_text = Some("No track is currently playing".into());
            self.state.synced_lyrics = None;
        }
    }
}
