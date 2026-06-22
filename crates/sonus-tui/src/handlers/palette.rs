use std::sync::mpsc;

use crate::app::App;
use sonus_core::player::{PlayerCommand, PlayerEvent};
use crate::state::app_state::Focus;
use crate::state::palette::{ConfirmAction, PaletteMode};


impl App {
    pub(crate) fn open_command_palette(&mut self) {
        self.state.palette_visible = true;
        self.state.palette_mode = PaletteMode::CommandSelection;
        self.state.palette_input.clear();
        self.state.palette_selected = 0;
        self.filter_palette_items();
    }

    pub(crate) fn open_context_menu(&mut self) {
        let origin = self.state.focus.clone();
        if origin == Focus::Tracklist {
            self.state.palette_track = self.state.active_track();
        } else if origin == Focus::Queue {
            self.state.palette_track = self.state.queue.get(self.state.queue_index).cloned();
        }
        self.state.palette_visible = true;
        self.state.palette_mode = PaletteMode::ContextActions;
        self.state.palette_input.clear();
        self.state.palette_selected = 0;
        self.filter_palette_items();
    }

    pub(crate) fn close_command_palette(&mut self) {
        self.state.palette_visible = false;
        self.state.palette_input.clear();
        self.state.palette_selected = 0;
        self.state.palette_items.clear();
        self.state.cached_playlists = None;
    }

    pub(crate) fn filter_palette_items(&mut self) {
        let input = self.state.palette_input.to_lowercase();
        match self.state.palette_mode {
            PaletteMode::CommandSelection => {
                self.state.palette_items = crate::state::palette::AVAILABLE_COMMANDS
                    .iter()
                    .filter(|cmd| cmd.name.to_lowercase().contains(&input) || cmd.description.to_lowercase().contains(&input))
                    .map(|cmd| format!("{} - {}", cmd.name, cmd.description))
                    .collect();
            }
            PaletteMode::CreatePlaylistInput => {
                self.state.palette_items = vec![];
            }
            PaletteMode::DeletePlaylistSelection => {
                let playlists = self.state.cached_playlists.get_or_insert_with(|| self.db.get_playlists().unwrap_or_default());
                self.state.palette_items = playlists.iter()
                    .map(|(_, name)| name.clone())
                    .filter(|name| name.to_lowercase().contains(&input))
                    .collect();
            }
            PaletteMode::AddToPlaylistSelection => {
                let playlists = self.state.cached_playlists.get_or_insert_with(|| self.db.get_playlists().unwrap_or_default());
                self.state.palette_items = playlists.iter()
                    .map(|(_, name)| name.clone())
                    .filter(|name| name.to_lowercase().contains(&input))
                    .collect();
            }
            PaletteMode::SeekInput => {
                self.state.palette_items = vec![];
            }
            PaletteMode::SpotifyImportInput => {
                self.state.palette_items = vec![];
            }
            PaletteMode::ThemeSelection => {
                self.state.palette_items = crate::config::PREDEFINED_THEMES
                    .iter()
                    .map(|t| t.name.to_string())
                    .filter(|name| name.to_lowercase().contains(&input))
                    .collect();
            }
            PaletteMode::Confirmation(_) => {
                self.state.palette_items = vec![
                    "Yes, proceed".to_string(),
                    "No, cancel".to_string(),
                ];
            }
            PaletteMode::ContextActions => {
                let mut options = Vec::new();
                match self.state.focus {
                    Focus::Sidebar => {
                        if !self.state.sidebar_items.is_empty() {
                            options.push("play playlist: clear queue and play this playlist now".to_string());
                            options.push("add to queue: add all tracks in this playlist to queue".to_string());
                            
                            // Check if it's a custom local playlist (index >= 4 because first 4 are Search, For You, History, Top Artists)
                            if self.state.sidebar_index >= 4 {
                                options.push("delete playlist: delete this local playlist".to_string());
                            }
                        }
                    }
                    Focus::Tracklist => {
                        if !self.state.tracks.is_empty() {
                            options.push("play now: play this track immediately".to_string());
                            options.push("add to queue: add this track to the end of the queue".to_string());
                            options.push("play next: queue this track to play next".to_string());
                            options.push("add to playlist: add this track to a local playlist".to_string());
                            
                            // Show "go to artist" if the track has a valid artist
                            if let Some(track) = self.state.active_track() {
                                if !track.artist.trim().is_empty() {
                                    options.push(format!("go to artist: view tracks for {}", track.artist));
                                }
                            }
                            
                            // Show "remove from playlist" if currently viewing a local playlist
                            if self.state.active_page == crate::state::app_state::ActivePage::Library && self.state.active_local_playlist_id.is_some() {
                                options.push("remove from playlist: remove this track from the current playlist".to_string());
                            }
                            
                            // Show "remove from history" if viewing history
                            if self.state.active_page == crate::state::app_state::ActivePage::Explore && self.state.explore_section == crate::state::app_state::ExploreSection::History {
                                options.push("remove from history: remove this track from history".to_string());
                            }
                        }
                    }
                    Focus::Queue => {
                        if !self.state.queue.is_empty() {
                            options.push("play now: play this track immediately".to_string());
                            options.push("play next: queue this track to play next".to_string());
                            options.push("add to playlist: add this track to a local playlist".to_string());
                            options.push("remove from queue: remove this track from the queue".to_string());
                        }
                    }
                    _ => {}
                }
                self.state.palette_items = options
                    .into_iter()
                    .filter(|opt| opt.to_lowercase().contains(&input))
                    .collect();
            }
        }
        let max = self.state.palette_items.len().saturating_sub(1);
        if self.state.palette_selected > max {
            self.state.palette_selected = max;
        }
    }

    pub(crate) fn handle_palette_key(&mut self, key: crossterm::event::KeyEvent, player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.close_command_palette();
            }
            crossterm::event::KeyCode::Up => {
                if self.state.palette_selected > 0 {
                    self.state.palette_selected -= 1;
                }
            }
            crossterm::event::KeyCode::Down => {
                let max = self.state.palette_items.len().saturating_sub(1);
                if self.state.palette_selected < max {
                    self.state.palette_selected += 1;
                }
            }
            crossterm::event::KeyCode::Enter => {
                self.execute_palette_action(player_cmd_tx);
            }
            crossterm::event::KeyCode::Backspace => {
                if let PaletteMode::Confirmation(_) = self.state.palette_mode {
                    // Ignore backspace in confirmation mode
                } else {
                    self.state.palette_input.pop();
                    self.filter_palette_items();
                }
            }
            crossterm::event::KeyCode::Char(c) => {
                if let PaletteMode::Confirmation(_) = self.state.palette_mode {
                    if c == 'y' || c == 'Y' {
                        self.state.palette_selected = 0;
                        self.execute_palette_action(player_cmd_tx);
                    } else if c == 'n' || c == 'N' {
                        self.close_command_palette();
                    }
                } else {
                    self.state.palette_input.push(c);
                    self.filter_palette_items();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn execute_palette_action(&mut self, _player_cmd_tx: &mpsc::Sender<PlayerCommand>) {
        match self.state.palette_mode {
            PaletteMode::CommandSelection => {
                let selected = self.state.palette_selected;
                if selected >= self.state.palette_items.len() {
                    return;
                }
                let selection = self.state.palette_items[selected].clone();
                
                if selection.starts_with("theme: set") {
                    self.state.palette_mode = PaletteMode::ThemeSelection;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                } else if selection.starts_with("playlist: create") {
                    self.state.palette_mode = PaletteMode::CreatePlaylistInput;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                } else if selection.starts_with("playlist: delete") {
                    self.state.palette_mode = PaletteMode::DeletePlaylistSelection;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                } else if selection.starts_with("playlist: import spotify") {
                    self.state.palette_mode = PaletteMode::SpotifyImportInput;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                } else if selection.starts_with("queue: clear") {
                    self.state.palette_mode = PaletteMode::Confirmation(ConfirmAction::ClearQueue);
                    self.state.palette_selected = 1;
                    self.filter_palette_items();
                } else if selection.starts_with("history: clear") {
                    self.state.palette_mode = PaletteMode::Confirmation(ConfirmAction::ClearHistory);
                    self.state.palette_selected = 1;
                    self.filter_palette_items();
                } else if selection.starts_with("seek") {
                    self.state.palette_mode = PaletteMode::SeekInput;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                } else if selection.starts_with("cache: clear") {
                    self.state.palette_mode = PaletteMode::Confirmation(ConfirmAction::ClearCache);
                    self.state.palette_selected = 1;
                    self.filter_palette_items();
                } else if selection.starts_with("view: toggle lyrics") {
                    self.toggle_lyrics();
                    self.close_command_palette();
                } else if selection.starts_with("view: toggle queue") {
                    self.toggle_queue();
                    self.close_command_palette();
                } else if selection.starts_with("view: toggle history") {
                    self.state.active_page = crate::state::app_state::ActivePage::Explore;
                    self.state.explore_section = crate::state::app_state::ExploreSection::History;
                    if let Some(ref ytm) = self.ytm {
                        let ytm_clone = std::sync::Arc::clone(ytm);
                        self.load_explore_data(&ytm_clone);
                    }
                    self.state.focus = Focus::Tracklist;
                    self.close_command_palette();
                } else if selection.starts_with("view: toggle help") {
                    self.toggle_help();
                    self.close_command_palette();
                } else if selection.starts_with("layout: toggle resize") {
                    self.state.resize_mode = !self.state.resize_mode;
                    self.close_command_palette();
                } else if selection.starts_with("playback: play/pause") {
                    self.toggle_play_pause(_player_cmd_tx);
                    self.close_command_palette();
                } else if selection.starts_with("playback: stop") {
                    self.stop_playback();
                    self.close_command_palette();
                } else if selection.starts_with("playback: next") {
                    if !self.state.queue.is_empty() {
                        let _ = _player_cmd_tx.send(PlayerCommand::Stop);
                        self.play_next_in_queue();
                    } else {
                        self.state.status_message = Some("Queue is empty".to_string());
                    }
                    self.close_command_palette();
                } else if selection.starts_with("playback: previous") {
                    if !self.state.history.is_empty() || self.state.player.current_track.is_some() {
                        let _ = _player_cmd_tx.send(PlayerCommand::Stop);
                        self.play_previous_in_queue();
                    } else {
                        self.state.status_message = Some("No previous track history".to_string());
                    }
                    self.close_command_palette();
                } else if selection.starts_with("playback: toggle shuffle") {
                    self.toggle_shuffle();
                    self.close_command_palette();
                } else if selection.starts_with("playback: toggle auto play") {
                    self.state.auto_play = !self.state.auto_play;
                    self.state.status_message = Some(format!("Auto-play is now {}", if self.state.auto_play { "on" } else { "off" }));
                    self.close_command_palette();
                } else if selection.starts_with("playback: toggle repeat") {
                    self.cycle_repeat_mode();
                    self.close_command_palette();
                } else if selection.starts_with("playback: repeat: none") {
                    self.state.player.repeat = crate::state::app_state::RepeatMode::None;
                    self.state.status_message = Some("Repeat mode disabled".to_string());
                    self.update_mpris_state();
                    self.close_command_palette();
                } else if selection.starts_with("playback: repeat: all") {
                    self.state.player.repeat = crate::state::app_state::RepeatMode::All;
                    self.state.status_message = Some("Repeat mode set to Repeat All".to_string());
                    self.update_mpris_state();
                    self.close_command_palette();
                } else if selection.starts_with("playback: repeat: one") {
                    self.state.player.repeat = crate::state::app_state::RepeatMode::One;
                    self.state.status_message = Some("Repeat mode set to Repeat One".to_string());
                    self.update_mpris_state();
                    self.close_command_palette();
                }
            }
            PaletteMode::CreatePlaylistInput => {
                let name = self.state.palette_input.trim().to_string();
                if !name.is_empty() {
                    match self.db.create_playlist(&name) {
                        Ok(_) => {
                            self.state.status_message = Some(format!("Playlist '{}' created", name));
                            self.load_playlists_to_sidebar();
                        }
                        Err(e) => {
                            self.state.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                self.close_command_palette();
            }
            PaletteMode::DeletePlaylistSelection => {
                let selected = self.state.palette_selected;
                if selected < self.state.palette_items.len() {
                    let playlist_name = self.state.palette_items[selected].clone();
                    let playlists = self.state.cached_playlists.get_or_insert_with(|| self.db.get_playlists().unwrap_or_default());
                    if let Some((id, _)) = playlists.iter().find(|(_, name)| name == &playlist_name) {
                        self.state.palette_mode = PaletteMode::Confirmation(ConfirmAction::DeletePlaylist { id: *id });
                        self.state.palette_selected = 1;
                        self.filter_palette_items();
                    } else {
                        self.close_command_palette();
                    }
                } else {
                    self.close_command_palette();
                }
            }
            PaletteMode::AddToPlaylistSelection => {
                let selected = self.state.palette_selected;
                if selected < self.state.palette_items.len() {
                    let playlist_name = self.state.palette_items[selected].clone();
                    let playlists = self.state.cached_playlists.get_or_insert_with(|| self.db.get_playlists().unwrap_or_default());
                    if let Some((id, _)) = playlists.iter().find(|(_, name)| name == &playlist_name) {
                        if let Some(track) = self.state.palette_track.take() {
                            match self.db.add_track_to_playlist(*id, track.as_ref()) {
                                Ok(_) => {
                                    self.state.status_message = Some(format!("Track added to playlist '{}'", playlist_name));
                                }
                                Err(e) => {
                                    self.state.status_message = Some(format!("Error: {}", e));
                                }
                            }
                        }
                    }
                }
                self.close_command_palette();
            }
            PaletteMode::SeekInput => {
                let input = self.state.palette_input.trim().to_string();
                if !input.is_empty() {
                    let parsed_secs = sonus_core::util::parse_time_string(&input);

                    if let Some(target_secs) = parsed_secs {
                        let duration = self.state.player.duration;
                        if duration > 0.0 {
                            let target = target_secs.clamp(0.0, duration);
                            let _ = _player_cmd_tx.send(PlayerCommand::Seek(target));
                            self.state.status_message = Some(format!("Seeked to {}", input));
                        }
                    } else {
                        self.state.status_message = Some("Invalid seek format. Use seconds or mm:ss".to_string());
                    }
                }
                self.close_command_palette();
            }
            PaletteMode::SpotifyImportInput => {
                let input = self.state.palette_input.trim().to_string();
                if !input.is_empty() {
                    self.start_spotify_import(input);
                }
                self.close_command_palette();
            }
            PaletteMode::ThemeSelection => {
                let selected = self.state.palette_selected;
                if selected < self.state.palette_items.len() {
                    let theme_name = self.state.palette_items[selected].clone();
                    match crate::config::update_theme(&theme_name) {
                        Ok(_) => {
                            self.state.status_message = Some(format!("Theme set to '{}'", theme_name));
                        }
                        Err(e) => {
                            self.state.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                self.close_command_palette();
            }
            PaletteMode::Confirmation(action) => {
                let selected = self.state.palette_selected;
                if selected == 0 {
                    match action {
                        ConfirmAction::ClearQueue => {
                            self.state.queue.clear();
                            self.state.queue_index = 0;
                            self.state.status_message = Some("Queue cleared".to_string());
                        }
                        ConfirmAction::ClearHistory => {
                            let _ = self.db.clear_history_tracks();
                            self.state.history.clear();
                            self.state.history_index = 0;
                            self.state.status_message = Some("History cleared".to_string());
                        }
                        ConfirmAction::ClearCache => {
                            let evt_tx = self.player_evt_tx.clone();
                            self.state.status_message = Some("Clearing cache...".to_string());
                            tokio::spawn(async move {
                                let cache_dir = sonus_core::player::cache::get_cache_dir();
                                let mut size_freed = 0u64;
                                if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                                    for entry in entries.flatten() {
                                        if let Ok(metadata) = entry.metadata() {
                                            if metadata.is_file() {
                                                size_freed += metadata.len();
                                                let _ = std::fs::remove_file(entry.path());
                                            }
                                        }
                                    }
                                }
                                let freed_mb = size_freed as f64 / 1_000_000.0;
                                if let Some(evt_tx) = evt_tx {
                                    let msg = format!("Cache cleared ({:.1} MB freed)", freed_mb);
                                    let _ = evt_tx.send(PlayerEvent::StatusMessage(msg));
                                }
                            });
                        }
                        ConfirmAction::DeletePlaylist { id } => {
                            let playlists = self.state.cached_playlists.get_or_insert_with(|| self.db.get_playlists().unwrap_or_default());
                            if let Some((_, playlist_name)) = playlists.iter().find(|(pid, _)| *pid == id).cloned() {
                                match self.db.delete_playlist(id) {
                                    Ok(_) => {
                                        self.state.status_message = Some(format!("Deleted playlist '{}'", playlist_name));
                                        self.state.cached_playlists = None;
                                        self.load_playlists_to_sidebar();
                                        
                                        if self.state.view_title.starts_with(&playlist_name) {
                                            self.state.view_title.clear();
                                            self.state.tracks.clear();
                                            self.state.is_search_results = false;
                                            self.state.track_index = 0;
                                            self.state.track_offset = 0;
                                        }
                                    }
                                    Err(e) => {
                                        self.state.status_message = Some(format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
                self.close_command_palette();
            }
            PaletteMode::ContextActions => {
                let selected = self.state.palette_selected;
                if selected >= self.state.palette_items.len() {
                    return;
                }
                let selection = self.state.palette_items[selected].clone();
                
                if selection.starts_with("play playlist") {
                    if self.state.focus == Focus::Sidebar {
                        if let Some(item) = self.state.sidebar_items.get(self.state.sidebar_index).cloned() {
                            if let Some(local_id) = item.local_playlist_id {
                                if let Ok(tracks) = self.db.get_playlist_tracks(local_id) {
                                    self.state.queue = tracks.into_iter().map(std::sync::Arc::new).collect();
                                    self.state.queue_index = 0;
                                    self.play_next_in_queue();
                                }
                            }
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("delete playlist") {
                    if self.state.focus == Focus::Sidebar {
                        if let Some(item) = self.state.sidebar_items.get(self.state.sidebar_index).cloned() {
                            if let Some(local_id) = item.local_playlist_id {
                                self.state.palette_mode = PaletteMode::Confirmation(ConfirmAction::DeletePlaylist { id: local_id });
                                self.state.palette_selected = 1;
                                self.filter_palette_items();
                                return;
                            }
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("add to queue") {
                    if self.state.focus == Focus::Sidebar {
                        if let Some(item) = self.state.sidebar_items.get(self.state.sidebar_index).cloned() {
                            if let Some(local_id) = item.local_playlist_id {
                                if let Ok(tracks) = self.db.get_playlist_tracks(local_id) {
                                    for t in tracks {
                                        if !self.state.queue.iter().any(|qt| qt.video_id == t.video_id) {
                                            self.state.queue.push(std::sync::Arc::new(t));
                                        }
                                    }
                                    self.state.status_message = Some("Playlist tracks added to queue".into());
                                }
                            } else if let Some(pid) = &item.playlist_id {
                                if self.pending_api.is_none() {
                                    let ytm = match &self.ytm {
                                        Some(y) => std::sync::Arc::clone(y),
                                        None => {
                                            self.close_command_palette();
                                            return;
                                        }
                                    };
                                    let pid = pid.clone();
                                    let (tx, rx) = tokio::sync::oneshot::channel();
                                    self.pending_api = Some(rx);
                                    tokio::spawn(async move {
                                        match ytm.get_playlist_tracks(&pid).await {
                                            Ok(tracks) => { let _ = tx.send(crate::app::ApiResult::AddPlaylistToQueue(tracks)); }
                                            Err(e) => { let _ = tx.send(crate::app::ApiResult::Error(e)); }
                                        }
                                    });
                                }
                            }
                        }
                    } else {
                        if let Some(track) = self.state.palette_track.take() {
                            self.add_to_queue(track);
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("play now") {
                    match self.state.focus {
                        Focus::Tracklist => self.play_selected_track(_player_cmd_tx),
                        Focus::Queue => self.play_from_queue(_player_cmd_tx),
                        _ => {}
                    }
                    self.close_command_palette();
                } else if selection.starts_with("play next") {
                    if let Some(track) = self.state.palette_track.take() {
                        if self.state.queue.is_empty() {
                            self.state.queue.push(track.clone());
                            self.state.queue_index = 0;
                            self.play_track_item(&track, _player_cmd_tx, true);
                        } else {
                            let insert_pos = (self.state.queue_index + 1).min(self.state.queue.len());
                            self.state.queue.insert(insert_pos, track);
                            self.state.status_message = Some("Queued to play next".to_string());
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("go to artist") {
                    if let Some(track) = self.state.palette_track.take() {
                        let artist_name = track.artist.clone();
                        self.state.active_page = crate::state::app_state::ActivePage::Search;
                        self.state.focus = Focus::Search;
                        self.state.search_query = artist_name.clone();
                        self.state.is_search_results = false;
                        self.state.tracks.clear();
                        if let Some(ref ytm) = self.ytm {
                            let ytm = std::sync::Arc::clone(ytm);
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            self.pending_api = Some(rx);
                            self.state.status_message = Some(format!("Searching for {}...", artist_name));
                            self.state.focus = Focus::Tracklist;
                            tokio::spawn(async move {
                                match ytm.search_all(&artist_name).await {
                                    Ok(tracks) => { let _ = tx.send(crate::app::ApiResult::Tracks(tracks)); }
                                    Err(e) => { let _ = tx.send(crate::app::ApiResult::Error(e)); }
                                }
                            });
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("remove from playlist") {
                    if let Some(playlist_id) = self.state.active_local_playlist_id {
                        if let Some(track) = self.state.palette_track.take() {
                            let vid = track.video_id.as_deref().unwrap_or("");
                            if self.db.remove_track_from_playlist(playlist_id, vid, &track.title, &track.artist).is_ok() {
                                self.state.status_message = Some(format!("Removed '{}' from playlist", track.title));
                                if let Ok(tracks) = self.db.get_playlist_tracks(playlist_id) {
                                    self.state.tracks = tracks.into_iter().map(std::sync::Arc::new).collect();
                                    let max = self.state.tracks.len().saturating_sub(1);
                                    if self.state.track_index > max {
                                        self.state.track_index = max;
                                    }
                                }
                            }
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("remove from history") {
                    if let Some(track) = self.state.palette_track.take() {
                        let vid = track.video_id.as_deref().unwrap_or("");
                        if self.db.delete_history_track(vid, &track.title, &track.artist).is_ok() {
                            self.state.status_message = Some(format!("Removed '{}' from history", track.title));
                            if let Ok(history) = self.db.get_history_tracks() {
                                self.state.history = history.into_iter().map(std::sync::Arc::new).collect();
                                let max = self.state.history.len().saturating_sub(1);
                                if self.state.track_index > max {
                                    self.state.track_index = max;
                                }
                            }
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("remove from queue") {
                    if !self.state.queue.is_empty() {
                        let idx = self.state.queue_index;
                        if idx < self.state.queue.len() {
                            let removed = self.state.queue.remove(idx);
                            self.state.status_message = Some(format!("Removed '{}' from queue", removed.title));
                            let max = self.state.queue.len().saturating_sub(1);
                            if self.state.queue_index > max {
                                self.state.queue_index = max;
                            }
                        }
                    }
                    self.close_command_palette();
                } else if selection.starts_with("add to playlist") {
                    self.state.palette_mode = PaletteMode::AddToPlaylistSelection;
                    self.state.palette_input.clear();
                    self.state.palette_selected = 0;
                    self.filter_palette_items();
                }
            }
        }
    }
}
