use sonus_core::player::PlayerCommand;
use sonus_core::types::RepeatMode;
use crate::app::App;
use crate::state::app_state::PlayStatus;

impl App {
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
                self.toggle_play_pause(&tx);
            }
            MprisCommand::Stop => {
                self.stop_playback();
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
                let track = sonus_core::types::TrackItem {
                    index: 0,
                    title: "Opening...".to_string(),
                    artist: "".to_string(),
                    duration: "0:00".to_string(),
                    duration_secs: 0.0,
                    is_playing: false,
                    video_id: Some(video_id),
                    album: None,
                    category: sonus_core::types::TrackCategory::Song,
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
