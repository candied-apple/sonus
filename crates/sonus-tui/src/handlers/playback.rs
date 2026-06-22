use std::sync::mpsc;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::app::{ApiResult, App};
use sonus_core::player::PlayerCommand;
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

    pub(crate) fn adjust_volume(&mut self, delta: f64, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        let vol = (self.state.player.volume * 20.0 + delta).clamp(0.0, 20.0) / 20.0;
        self.state.player.volume = vol;
        let _ = player_cmd_tx.send(PlayerCommand::SetVolume(vol));
        crate::config::update_default_volume(vol);
        self.update_mpris_state();
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

    pub(crate) fn toggle_play_pause(&mut self, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        match self.state.player.status {
            PlayStatus::Playing => {
                let _ = player_cmd_tx.send(PlayerCommand::Pause);
                self.state.player.status = PlayStatus::Paused;
            }
            PlayStatus::Paused => {
                let _ = player_cmd_tx.send(PlayerCommand::Resume);
                self.state.player.status = PlayStatus::Playing;
            }
            PlayStatus::Stopped => {
                self.play_selected_track(player_cmd_tx);
            }
        }
        self.update_mpris_state();
    }

    pub(crate) fn stop_playback(&mut self) {
        if let Some(ref tx) = self.player_cmd_tx {
            let _ = tx.send(PlayerCommand::Stop);
        }
        self.state.player.status = PlayStatus::Stopped;
        self.state.player.current_track = None;
        self.state.player.current_video_id = None;
        self.state.player.position = 0.0;
        self.state.player.duration = 0.0;
        self.cover_image = None;
        self.current_cover_video_id = None;
        self.update_mpris_state();
    }

    pub(crate) fn current_track_item(&self) -> Option<TrackItem> {
        let video_id = self.state.player.current_video_id.clone()?;
        let (title, artist) = self.state.player.current_track.clone()?;
        let duration_secs = self.state.player.duration;
        let duration = crate::ui::components::format_duration(duration_secs);
        Some(TrackItem {
            index: 0,
            title,
            artist,
            duration,
            duration_secs,
            is_playing: true,
            video_id: Some(video_id),
            album: None,
            category: crate::state::app_state::TrackCategory::Song,
        })
    }
}
