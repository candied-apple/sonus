use std::sync::mpsc;
use std::sync::Arc;

use crate::app::App;
use sonus_core::player::PlayerCommand;
use crate::state::app_state::TrackItem;

impl App {
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
            if let Some(current_track) = self.current_track_item() {
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
            self.stop_playback();
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
            if let Some(current_track) = self.current_track_item() {
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
            if let Some(current_track) = self.current_track_item() {
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

    pub(crate) fn toggle_shuffle(&mut self) {
        self.state.player.shuffle = !self.state.player.shuffle;
        self.state.status_message = Some(format!(
            "Shuffle: {}",
            if self.state.player.shuffle { "ON" } else { "OFF" }
        ));
        self.update_mpris_state();
    }

    pub(crate) fn cycle_repeat_mode(&mut self) {
        self.state.player.repeat = match self.state.player.repeat {
            crate::state::app_state::RepeatMode::None => crate::state::app_state::RepeatMode::All,
            crate::state::app_state::RepeatMode::All => crate::state::app_state::RepeatMode::One,
            crate::state::app_state::RepeatMode::One => crate::state::app_state::RepeatMode::None,
        };
        self.state.status_message = Some(format!(
            "Repeat: {:?}",
            self.state.player.repeat
        ));
        self.update_mpris_state();
    }
}

fn random_index(max: usize) -> usize {
    if max <= 1 {
        return 0;
    }
    use rand::Rng;
    rand::thread_rng().gen_range(0..max)
}
