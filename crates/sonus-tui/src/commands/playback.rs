use std::sync::mpsc;
use std::sync::Arc;

use tokio::sync::oneshot;

use sonus_core::api::client::shared_http_client;
use crate::app::{ApiResult, App, parse_lrc};
use sonus_core::player::PlayerCommand;
use sonus_core::types::{RepeatMode, TrackCategory};
use crate::state::app_state::{PlayStatus, TrackItem};

impl App {
    pub(crate) fn play_track_item(&mut self, track: &TrackItem, player_cmd_tx: &mpsc::Sender<PlayerCommand>, archive_to_history: bool) {
        let video_id = match &track.video_id {
            Some(id) => id.clone(),
            None => {
                self.state.status_message = Some("No video ID available".into());
                return;
            }
        };

        // Archive current track to history immediately
        if archive_to_history {
            if self.state.history.first().and_then(|t| t.video_id.as_ref()) != Some(&video_id) {
                let limit = crate::config::history_limit();
                let history_track = Arc::new(TrackItem {
                    index: 0,
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    duration: track.duration.clone(),
                    duration_secs: track.duration_secs,
                    is_playing: false,
                    video_id: Some(video_id.clone()),
                    album: track.album.clone(),
                    category: track.category,
                });
                self.state.history.insert(0, history_track.clone());
                if self.state.history.len() > limit {
                    self.state.history.truncate(limit);
                }
                let _ = self.db.add_history_track(&history_track, limit);
                self.state.explore_loaded = false;

                if let Some(ref ytm) = self.ytm {
                    let album_is_placeholder = match track.album.as_deref() {
                        None | Some("") | Some("-") => true,
                        _ => false,
                    };
                    if album_is_placeholder {
                        let db = self.db.clone();
                        let ytm = Arc::clone(ytm);
                        let vid = video_id.clone();
                        let title = track.title.clone();
                        let artist = track.artist.clone();
                        tokio::spawn(async move {
                            if let Ok(results) = ytm.search_songs(&format!("{} {}", title, artist)).await {
                                if let Some(matching) = results.into_iter().find(|t| t.video_id.as_ref() == Some(&vid)) {
                                    if let Some(album_name) = matching.album {
                                        if !album_name.is_empty() && album_name != "-" {
                                            let _ = db.update_track_album(&vid, &album_name);
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }

        let dur = track.duration_secs;
        let _ = player_cmd_tx.send(PlayerCommand::Play {
            video_id: video_id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            duration_secs: dur,
        });

        self.state.player.current_video_id = Some(video_id.clone());
        self.state.player.duration = dur;
        self.state.player.status = PlayStatus::Playing;
        self.state.player.current_track = Some((track.title.clone(), track.artist.clone()));

        self.update_mpris_state();

        self.state.lyrics_text = None;
        self.fetch_cover_image(video_id);
        if self.state.lyrics_visible {
            self.fetch_lyrics_for_current_track();
        }
    }

    pub(crate) fn play_selected_track(&mut self, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        let track = match self.state.active_track() {
            Some(t) => Arc::clone(&t),
            None => return,
        };

        let active_list = self.state.active_track_list();
        let local_idx = self.state.active_local_index();
        let is_single_track = !self.state.view_title.ends_with("  Playlist") && self.state.active_page != crate::state::app_state::ActivePage::Explore;

        if !is_single_track {
            if local_idx + 1 < active_list.len() {
                self.state.queue = active_list[local_idx + 1..].to_vec();
            } else {
                self.state.queue.clear();
            }
            self.state.queue_index = 0;
        }

        self.state.history_cursor = 0;
        self.play_track_item(&track, player_cmd_tx, true);

        if is_single_track && self.state.auto_play {
            if let Some(ref vid) = track.video_id {
                self.fetch_recommendations(vid);
            }
        }
    }

    pub(crate) fn play_from_queue(&mut self, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        if self.state.queue.is_empty() {
            return;
        }
        let idx = self.state.queue_index;
        if idx >= self.state.queue.len() {
            return;
        }

        let skipped = self.state.queue.drain(0..idx).collect::<Vec<_>>();
        let track = self.state.queue.remove(0);

        let limit = crate::config::history_limit();
        let mut added_any = false;
        for t in skipped.into_iter().rev() {
            self.state.history.insert(0, t.clone());
            let _ = self.db.add_history_track(&t, limit);
            added_any = true;
        }
        if added_any {
            self.state.explore_loaded = false;
        }
        if self.state.history.len() > limit {
            self.state.history.truncate(limit);
        }

        self.state.queue_index = 0;
        self.state.history_cursor = 0;
        self.play_track_item(&track, player_cmd_tx, true);
    }



    pub(crate) fn play_next_in_queue(&mut self) {
        let cmd_tx = match self.player_cmd_tx.clone() {
            Some(tx) => tx,
            None => return,
        };

        if self.state.player.repeat == crate::state::app_state::RepeatMode::One {
            if let (Some(video_id), Some((title, artist))) = (
                self.state.player.current_video_id.clone(),
                self.state.player.current_track.clone(),
            ) {
                let duration_secs = self.state.player.duration;
                let duration = crate::ui::components::format_duration(duration_secs);
                let current_track = TrackItem {
                    index: 0,
                    title,
                    artist,
                    duration,
                    duration_secs,
                    is_playing: true,
                    video_id: Some(video_id),
                    album: None,
                    category: crate::state::app_state::TrackCategory::Song,
                };
                self.play_track_item(&current_track, &cmd_tx, true);
                return;
            }
        }

        if self.state.queue.is_empty() {
            if self.state.player.repeat == crate::state::app_state::RepeatMode::All {
                if !self.state.tracks.is_empty() {
                    self.state.queue = self.state.tracks.clone();
                }
            }
        }

        if self.state.queue.is_empty() {
            let _ = cmd_tx.send(PlayerCommand::Stop);
            self.state.player.status = PlayStatus::Stopped;
            self.state.player.current_track = None;
            self.state.player.current_video_id = None;
            self.state.player.position = 0.0;
            self.state.player.duration = 0.0;
            self.cover_image = None;
            self.current_cover_video_id = None;
            return;
        }

        let archive = if self.state.history_cursor > 0 {
            self.state.history_cursor -= 1;
            false
        } else {
            true
        };

        let track = if self.state.player.shuffle {
            let idx = random_index(self.state.queue.len());
            self.state.queue.remove(idx)
        } else {
            self.state.queue.remove(0)
        };

        self.play_track_item(&track, &cmd_tx, archive);
    }

    pub(crate) fn play_previous_in_queue(&mut self) {
        let cmd_tx = match self.player_cmd_tx.clone() {
            Some(tx) => tx,
            None => return,
        };

        if self.state.player.repeat == crate::state::app_state::RepeatMode::One {
            if let (Some(video_id), Some((title, artist))) = (
                self.state.player.current_video_id.clone(),
                self.state.player.current_track.clone(),
            ) {
                let duration_secs = self.state.player.duration;
                let duration = crate::ui::components::format_duration(duration_secs);
                let current_track = TrackItem {
                    index: 0,
                    title,
                    artist,
                    duration,
                    duration_secs,
                    is_playing: true,
                    video_id: Some(video_id),
                    album: None,
                    category: crate::state::app_state::TrackCategory::Song,
                };
                self.play_track_item(&current_track, &cmd_tx, true);
                return;
            }
        }

        let prev_track_idx = self.state.history_cursor;
        if prev_track_idx < self.state.history.len() {
            let prev_track = Arc::clone(&self.state.history[prev_track_idx]);
            self.state.history_cursor += 1;

            if let (Some(video_id), Some((title, artist))) = (
                self.state.player.current_video_id.clone(),
                self.state.player.current_track.clone(),
            ) {
                let already_inserted = self.state.queue.first()
                    .and_then(|t| t.video_id.as_ref()) == Some(&video_id);

                if !already_inserted {
                    let duration_secs = self.state.player.duration;
                    let duration = crate::ui::components::format_duration(duration_secs);
                    let current_track = Arc::new(TrackItem {
                        index: 0,
                        title,
                        artist,
                        duration,
                        duration_secs,
                        is_playing: false,
                        video_id: Some(video_id),
                        album: None,
                        category: crate::state::app_state::TrackCategory::Song,
                    });
                    self.state.queue.insert(0, current_track);
                }
            }

            self.play_track_item(&prev_track, &cmd_tx, false);
        } else {
            if let (Some(video_id), Some((title, artist))) = (
                self.state.player.current_video_id.clone(),
                self.state.player.current_track.clone(),
            ) {
                let duration_secs = self.state.player.duration;
                let duration = crate::ui::components::format_duration(duration_secs);
                let current_track = TrackItem {
                    index: 0,
                    title,
                    artist,
                    duration,
                    duration_secs,
                    is_playing: true,
                    video_id: Some(video_id),
                    album: None,
                    category: crate::state::app_state::TrackCategory::Song,
                };
                self.play_track_item(&current_track, &cmd_tx, true);
            }
        }
    }

    pub(crate) fn add_to_queue(&mut self, track: Arc<TrackItem>) {
        if self.state.queue.iter().any(|t| t.video_id == track.video_id) {
            self.state.status_message = Some("Track already in queue".to_string());
            return;
        }
        self.state.queue.push(track);
        self.state.status_message = Some("Added to queue".to_string());
    }

    pub(crate) fn adjust_volume(&mut self, delta: f64, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        let vol = (self.state.player.volume * 20.0 + delta).clamp(0.0, 20.0) / 20.0;
        self.state.player.volume = vol;
        let _ = player_cmd_tx.send(PlayerCommand::SetVolume(vol));
        crate::config::update_default_volume(vol);
        self.update_mpris_state();
    }

    pub(crate) fn fetch_cover_image(&mut self, video_id: String) {
        if !crate::util::is_valid_video_id(&video_id) {
            self.current_cover_video_id = Some(video_id);
            return;
        }
        if self.current_cover_video_id.as_ref() == Some(&video_id) {
            return;
        }
        self.current_cover_video_id = Some(video_id.clone());
        self.cover_image = None;

        let (tx, rx) = oneshot::channel();
        self.pending_cover = Some(rx);

        tokio::spawn(async move {
            let url = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", video_id);
            match shared_http_client().get(url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if bytes.len() > 5 * 1024 * 1024 {
                                let _ = tx.send(Err("Image too large (>5MB)".to_string()));
                                return;
                            }
                            let _ = tx.send(Ok(bytes.to_vec()));
                            return;
                        }
                    }
                    let _ = tx.send(Err("Failed to read image bytes".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        });
    }

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
                            final_plain = final_plain.or(Some(y_plain));
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

    pub(crate) fn fetch_recommendations(&mut self, video_id: &str) {
        let ytm = match &self.ytm {
            Some(y) => Arc::clone(y),
            None => return,
        };

        if self.pending_api.is_none() {
            let vid = video_id.to_string();
            let (tx, rx) = oneshot::channel();
            self.pending_api = Some(rx);
            tokio::spawn(async move {
                match ytm.get_recommendations(&vid).await {
                    Ok(tracks) => { let _ = tx.send(ApiResult::Recommendations(tracks)); }
                    Err(e) => { let _ = tx.send(ApiResult::Error(e)); }
                }
            });
        }
    }

    pub(crate) fn update_mpris_state(&self) {
        let Some(ref mpris_state_lock) = self.mpris_state else { return; };
        let mut mpris = mpris_state_lock.write().unwrap();
        
        let player = &self.state.player;
        
        mpris.playback_status = match player.status {
            PlayStatus::Playing => "Playing".to_string(),
            PlayStatus::Paused => "Paused".to_string(),
            PlayStatus::Stopped => "Stopped".to_string(),
        };
        
        mpris.volume = player.volume;
        mpris.shuffle = player.shuffle;
        mpris.repeat = match player.repeat {
            RepeatMode::None => "None".to_string(),
            RepeatMode::All => "Playlist".to_string(),
            RepeatMode::One => "Track".to_string(),
        };
        
        mpris.can_go_next = !self.state.queue.is_empty();
        mpris.can_go_previous = !self.state.history.is_empty();
        
        mpris.position_micros = (player.position * 1_000_000.0) as i64;
        mpris.length_micros = (player.duration * 1_000_000.0) as i64;
        
        if let Some((ref title, ref artist)) = player.current_track {
            mpris.title = title.clone();
            mpris.artist = artist.clone();
        } else {
            mpris.title.clear();
            mpris.artist.clear();
        }
        
        mpris.video_id = player.current_video_id.clone();
        if let Some(ref vid) = player.current_video_id {
            let safe_vid = vid.replace('-', "_");
            mpris.track_id = format!("/org/mpris/MediaPlayer2/track/{}", safe_vid);
        } else {
            mpris.track_id.clear();
        }
        
        // Find album if possible
        if let Some(ref vid) = player.current_video_id {
            if let Ok(Some(album)) = self.db.get_album_by_video_id(vid) {
                mpris.album = Some(album);
            } else {
                mpris.album = None;
            }
        } else {
            mpris.album = None;
        }
        
        if let Some(ref tx) = self.mpris_signal_tx {
            let _ = tx.send(sonus_core::mpris::MprisSignal::PropertiesChanged);
        }
    }

    pub(crate) fn update_mpris_position(&self, pos: f64) {
        if let Some(ref mpris_state_lock) = self.mpris_state {
            if let Ok(mut mpris) = mpris_state_lock.write() {
                mpris.position_micros = (pos * 1_000_000.0) as i64;
            }
        }
    }

    pub(crate) fn handle_mpris_command(&mut self, cmd: sonus_core::mpris::MprisCommand) {
        use sonus_core::mpris::MprisCommand;
        let tx = match self.player_cmd_tx.clone() {
            Some(t) => t,
            None => return,
        };
        match cmd {
            MprisCommand::Play => {
                if self.state.player.status == PlayStatus::Paused {
                    let _ = tx.send(PlayerCommand::Resume);
                    self.state.player.status = PlayStatus::Playing;
                    self.update_mpris_state();
                } else if self.state.player.status == PlayStatus::Stopped {
                    self.play_selected_track(&tx);
                }
            }
            MprisCommand::Pause => {
                if self.state.player.status == PlayStatus::Playing {
                    let _ = tx.send(PlayerCommand::Pause);
                    self.state.player.status = PlayStatus::Paused;
                    self.update_mpris_state();
                }
            }
            MprisCommand::PlayPause => {
                match self.state.player.status {
                    PlayStatus::Playing => {
                        let _ = tx.send(PlayerCommand::Pause);
                        self.state.player.status = PlayStatus::Paused;
                    }
                    PlayStatus::Paused => {
                        let _ = tx.send(PlayerCommand::Resume);
                        self.state.player.status = PlayStatus::Playing;
                    }
                    PlayStatus::Stopped => {
                        self.play_selected_track(&tx);
                    }
                }
                self.update_mpris_state();
            }
            MprisCommand::Stop => {
                let _ = tx.send(PlayerCommand::Stop);
                self.state.player.status = PlayStatus::Stopped;
                self.state.player.current_track = None;
                self.state.player.current_video_id = None;
                self.state.player.position = 0.0;
                self.state.player.duration = 0.0;
                self.cover_image = None;
                self.current_cover_video_id = None;
                self.update_mpris_state();
            }
            MprisCommand::Next => {
                if !self.state.queue.is_empty() {
                    let _ = tx.send(PlayerCommand::Stop);
                    self.play_next_in_queue();
                    self.update_mpris_state();
                }
            }
            MprisCommand::Previous => {
                if !self.state.history.is_empty() {
                    let _ = tx.send(PlayerCommand::Stop);
                    self.play_previous_in_queue();
                    self.update_mpris_state();
                }
            }
            MprisCommand::Seek(secs) => {
                let _ = tx.send(PlayerCommand::Seek(secs));
                // Send seeked event to update media controls progress bar
                if let Some(ref tx_sig) = self.mpris_signal_tx {
                    let _ = tx_sig.send(sonus_core::mpris::MprisSignal::Seeked((secs * 1_000_000.0) as i64));
                }
            }
            MprisCommand::OpenUri(video_id) => {
                let track = TrackItem {
                    index: 0,
                    title: "Opening...".to_string(),
                    artist: "".to_string(),
                    duration: "0:00".to_string(),
                    duration_secs: 0.0,
                    is_playing: false,
                    video_id: Some(video_id),
                    album: None,
                    category: TrackCategory::Song,
                };
                self.play_track_item(&track, &tx, true);
            }
            MprisCommand::SetVolume(vol) => {
                let vol = vol.clamp(0.0, 1.0);
                self.state.player.volume = vol;
                let _ = tx.send(PlayerCommand::SetVolume(vol));
                crate::config::update_default_volume(vol);
                self.update_mpris_state();
            }
            MprisCommand::SetShuffle(shuffle) => {
                self.state.player.shuffle = shuffle;
                self.update_mpris_state();
            }
            MprisCommand::SetRepeat(repeat) => {
                self.state.player.repeat = match repeat.as_str() {
                    "Track" => RepeatMode::One,
                    "Playlist" => RepeatMode::All,
                    _ => RepeatMode::None,
                };
                self.update_mpris_state();
            }
        }
    }
}

fn random_index(max: usize) -> usize {
    if max <= 1 {
        return 0;
    }
    use rand::Rng;
    rand::thread_rng().gen_range(0..max)
}
