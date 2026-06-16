use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use tokio::sync::oneshot;

use sonus_core::api::client::YtmClient;
use crate::commands::import::ImportProgress;
use crate::ui;
use sonus_core::db::Db;
use sonus_core::player::{PlayerCommand, PlayerEvent};
use crate::state::app_state::{
    AppState, Focus,
};
use sonus_core::types::{PlayStatus, RepeatMode, SyncedLine, TrackItem, TrackCategory};
use crate::state::app_state::SearchTab;

pub(crate) enum ApiResult {
    Tracks(Vec<TrackItem>),
    AddPlaylistToQueue(Vec<TrackItem>),
    Lyrics {
        plain: Option<String>,
        synced: Option<String>,
    },
    Recommendations(Vec<TrackItem>),
    ExploreRecommendations(Vec<TrackItem>),
    Error(String),
}

pub(crate) struct App {
    pub(crate) state: AppState,
    pub(crate) should_quit: bool,
    pub(crate) pending_api: Option<oneshot::Receiver<ApiResult>>,
    pub(crate) player_cmd_tx: Option<mpsc::Sender<PlayerCommand>>,
    pub(crate) player_evt_tx: Option<tokio::sync::mpsc::UnboundedSender<PlayerEvent>>,
    pub(crate) ytm: Option<Arc<YtmClient>>,
    pub(crate) cover_image: Option<ratatui_image::protocol::StatefulProtocol>,
    pub(crate) picker: Option<ratatui_image::picker::Picker>,
    pub(crate) current_cover_video_id: Option<String>,
    pub(crate) pending_cover: Option<oneshot::Receiver<Result<Vec<u8>, String>>>,
    pub(crate) db: Db,
    pub(crate) import_rx: Option<tokio::sync::mpsc::Receiver<ImportProgress>>,
    pub(crate) pending_version_check: Option<oneshot::Receiver<String>>,
}

impl App {
    pub fn new() -> Self {
        let db_dir = dirs::data_dir()
            .map(|p| p.join("sonus"))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let _ = std::fs::create_dir_all(&db_dir);
        let db_path = db_dir.join("sonus.db");

        // Migrate existing database from CWD to standard user directory
        let cwd_db = std::path::Path::new("sonus.db");
        if !db_path.exists() && cwd_db.exists() {
            let _ = std::fs::copy(cwd_db, &db_path);
            let cwd_wal = std::path::Path::new("sonus.db-wal");
            if cwd_wal.exists() {
                let _ = std::fs::copy(cwd_wal, db_dir.join("sonus.db-wal"));
            }
            let cwd_shm = std::path::Path::new("sonus.db-shm");
            if cwd_shm.exists() {
                let _ = std::fs::copy(cwd_shm, db_dir.join("sonus.db-shm"));
            }
        }

        let db = Db::new(&db_path);
        db.init().expect("Failed to initialize SQLite database");
        let mut app = Self {
            state: AppState::default(),
            should_quit: false,
            pending_api: None,
            player_cmd_tx: None,
            player_evt_tx: None,
            ytm: None,
            cover_image: None,
            picker: None,
            current_cover_video_id: None,
            pending_cover: None,
            db,
            import_rx: None,
            pending_version_check: None,
        };
        app.load_playlists_to_sidebar();
        if let Ok(history) = app.db.get_history_tracks() {
            app.state.history = history.into_iter().map(Arc::new).collect();
        }
        app
    }


    pub async fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
        player_cmd_tx: mpsc::Sender<PlayerCommand>,
        mut player_evt_rx: tokio::sync::mpsc::UnboundedReceiver<PlayerEvent>,
        player_evt_tx: tokio::sync::mpsc::UnboundedSender<PlayerEvent>,
        ytm: Arc<YtmClient>,
    ) -> std::io::Result<()> {
        self.state.api_ready = true;
        self.player_cmd_tx = Some(player_cmd_tx.clone());
        self.player_evt_tx = Some(player_evt_tx.clone());
        self.ytm = Some(ytm.clone());

        self.state.active_page = crate::state::app_state::ActivePage::Explore;
        self.state.explore_section = crate::state::app_state::ExploreSection::ForYou;
        self.state.sidebar_index = 1;
        self.load_explore_data(&ytm);

        let (ver_tx, ver_rx) = oneshot::channel();
        self.pending_version_check = Some(ver_rx);
        tokio::spawn(async move {
            if let Ok(tag) = sonus_core::api::client::check_latest_release().await {
                let current = env!("CARGO_PKG_VERSION");
                let clean_tag = tag.trim_start_matches('v');
                if clean_tag != current {
                    let _ = ver_tx.send(tag);
                }
            }
        });

        if self.picker.is_none() {
            self.picker = ratatui_image::picker::Picker::from_query_stdio()
                .ok()
                .or_else(|| Some(ratatui_image::picker::Picker::halfblocks()));
        }

        let mut should_render = true;
        loop {
            if should_render {
                if self.state.needs_clear {
                    terminal.clear()?;
                    self.state.needs_clear = false;
                }
                terminal.draw(|f| ui::render(f, &mut self.state, &mut self.cover_image))?;
                should_render = false;
            }

            if self.should_quit {
                break;
            }

            while let Ok(evt) = player_evt_rx.try_recv() {
                self.handle_player_event(evt);
                should_render = true;
            }

            if let Some(rx) = &mut self.pending_cover {
                match rx.try_recv() {
                    Ok(Ok(bytes)) => {
                        self.pending_cover = None;
                        if let Ok(mut img) = image::load_from_memory(&bytes) {
                            let (w, h) = (img.width(), img.height());
                            let img = if w > h {
                                let aspect_ratio = w as f64 / h as f64;
                                let (x, y, crop_w, crop_h) = if (aspect_ratio - 4.0 / 3.0).abs() < 0.1 {
                                    let square_size = w * 9 / 16;
                                    let x_offset = (w - square_size) / 2;
                                    let y_offset = (h - square_size) / 2;
                                    (x_offset, y_offset, square_size, square_size)
                                } else {
                                    let square_size = h;
                                    let x_offset = (w - square_size) / 2;
                                    let y_offset = 0;
                                    (x_offset, y_offset, square_size, square_size)
                                };
                                img.crop(x, y, crop_w, crop_h)
                            } else {
                                img
                            };
                            if let Some(picker) = &mut self.picker {
                                let protocol = picker.new_resize_protocol(img);
                                self.cover_image = Some(protocol);
                                should_render = true;
                            }
                        }
                    }
                    Ok(Err(_)) => {
                        self.pending_cover = None;
                        self.cover_image = None;
                        should_render = true;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        self.pending_cover = None;
                        self.cover_image = None;
                        should_render = true;
                    }
                }
            }

            if let Some(rx) = &mut self.pending_version_check {
                match rx.try_recv() {
                    Ok(latest_version) => {
                        self.state.new_version_available = Some(latest_version);
                        self.pending_version_check = None;
                        should_render = true;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        self.pending_version_check = None;
                        should_render = true;
                    }
                }
            }

            if let Some(rx) = &mut self.pending_api {
                match rx.try_recv() {
                    Ok(ApiResult::Tracks(tracks)) => {
                        self.state.status_message = None;
                        self.state.tracks = tracks.into_iter().map(Arc::new).collect();
                        self.state.update_search_results_cache();
                        self.state.track_index = 0;
                        self.state.track_offset = 0;
                        self.pending_api = None;
                        should_render = true;
                    }
                    Ok(ApiResult::AddPlaylistToQueue(tracks)) => {
                        self.state.status_message = None;
                        let mut existing_ids: std::collections::HashSet<Option<String>> =
                            self.state.queue.iter().map(|qt| qt.video_id.clone()).collect();
                        for t in tracks {
                            if !existing_ids.contains(&t.video_id) {
                                existing_ids.insert(t.video_id.clone());
                                self.state.queue.push(Arc::new(t));
                            }
                        }
                        self.state.status_message = Some("Playlist tracks added to queue".to_string());
                        self.pending_api = None;
                        should_render = true;
                    }
                    Ok(ApiResult::Recommendations(tracks)) => {
                        self.state.status_message = None;
                        let current_video_id = self.state.player.current_video_id.clone();
                        let skip_ids: std::collections::HashSet<Option<String>> = std::iter::once(current_video_id)
                            .chain(self.state.queue.iter().map(|qt| qt.video_id.clone()))
                            .chain(self.state.history.iter().take(20).map(|ht| ht.video_id.clone()))
                            .collect();
                        let mut added = 0;
                        for t in tracks {
                            if added >= 20 {
                                break;
                            }
                            if skip_ids.contains(&t.video_id) {
                                continue;
                            }
                            self.state.queue.push(Arc::new(t));
                            added += 1;
                        }
                        if added > 0 {
                            self.state.status_message = Some(format!("Radio: {} track(s) added to queue", added));
                            if self.state.player.status == PlayStatus::Stopped {
                                self.play_next_in_queue();
                            }
                        }
                        self.pending_api = None;
                        should_render = true;
                    }
                    Ok(ApiResult::ExploreRecommendations(tracks)) => {
                        self.state.status_message = None;
                        let mut songs = Vec::new();
                        let mut videos = Vec::new();
                        let mut seen_ids = std::collections::HashSet::new();
                        for t in tracks {
                            if let Some(ref vid) = t.video_id {
                                if seen_ids.contains(vid) {
                                    continue;
                                }
                                seen_ids.insert(vid.clone());
                                if t.category == TrackCategory::Video {
                                    videos.push(t);
                                } else {
                                    songs.push(t);
                                }
                            }
                        }
                        use rand::seq::SliceRandom;
                        let mut rng = rand::thread_rng();
                        songs.shuffle(&mut rng);
                        videos.shuffle(&mut rng);
                        
                        songs.truncate(20);
                        videos.truncate(20);

                        let mut unique_tracks = songs;
                        unique_tracks.extend(videos);
                        unique_tracks.shuffle(&mut rng);

                        for (i, t) in unique_tracks.iter_mut().enumerate() {
                            t.index = i + 1;
                            let needs_album = match t.album.as_deref() {
                                None | Some("") | Some("-") => true,
                                _ => false,
                            };
                            if needs_album {
                                if let Some(ref vid) = t.video_id {
                                    if let Ok(Some(album_name)) = self.db.get_album_by_video_id(vid) {
                                        t.album = Some(album_name);
                                    }
                                }
                            }
                        }

                        let final_tracks: Vec<Arc<TrackItem>> = unique_tracks.into_iter().map(Arc::new).collect();
                        save_for_you_cache(&final_tracks);
                        self.state.explore_for_you = final_tracks;
                        self.pending_api = None;
                        should_render = true;
                    }
                    Ok(ApiResult::Lyrics { plain, synced }) => {
                        self.state.status_message = None;
                        self.state.lyrics_text = plain;
                        self.state.synced_lyrics = synced.map(|s| parse_lrc(&s));
                        self.pending_api = None;
                        should_render = true;
                    }
                    Ok(ApiResult::Error(e)) => {
                        self.state.status_message = Some(e);
                        self.pending_api = None;
                        should_render = true;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        self.pending_api = None;
                        should_render = true;
                    }
                }
            }

            let mut clear_rx = false;
            let mut reload_sidebar = false;
            if let Some(rx) = &mut self.import_rx {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        ImportProgress::Started { playlist_name, total } => {
                            self.state.spotify_import = Some(crate::state::app_state::SpotifyImportState {
                                playlist_name,
                                completed: 0,
                                total_tracks: total,
                                current_track_name: "Connecting...".to_string(),
                            });
                        }
                        ImportProgress::TrackProcessing { title, artist } => {
                            if let Some(imp) = &mut self.state.spotify_import {
                                imp.current_track_name = format!("{} - {}", title, artist);
                            }
                        }
                        ImportProgress::TrackResolved { completed, total } => {
                            if let Some(imp) = &mut self.state.spotify_import {
                                imp.completed = completed;
                                imp.current_track_name = format!("{} / {} tracks resolved", completed, total);
                            }
                        }
                        ImportProgress::Success { playlist_name, count } => {
                            self.state.spotify_import = None;
                            clear_rx = true;
                            self.state.status_message = Some(format!("Successfully imported '{}' ({} tracks)", playlist_name, count));
                            reload_sidebar = true;
                        }
                        ImportProgress::Error(e) => {
                            self.state.spotify_import = None;
                            clear_rx = true;
                            self.state.status_message = Some(format!("Import error: {}", e));
                        }
                    }
                    should_render = true;
                }
            }
            if clear_rx {
                self.import_rx = None;
            }
            if reload_sidebar {
                self.load_playlists_to_sidebar();
            }

            let poll_timeout = if self.state.player.status == PlayStatus::Playing || self.state.focus == Focus::Tracklist {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(250)
            };

            if event::poll(poll_timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            self.handle_key(key, &player_cmd_tx, &ytm);
                            should_render = true;
                        }
                    }
                    Event::Mouse(mouse) => {
                        if self.handle_mouse(mouse, &player_cmd_tx, &ytm) {
                            should_render = true;
                        }
                    }
                    Event::Resize(_, _) => {
                        should_render = true;
                        self.state.needs_clear = true;
                    }
                    _ => {}
                }
            } else {
                // If poll timed out and we are playing or in tracklist, force a render for animations/progress
                if self.state.player.status == PlayStatus::Playing || self.state.focus == Focus::Tracklist {
                    should_render = true;
                }
            }
        }

        Ok(())
    }

    fn handle_player_event(&mut self, evt: PlayerEvent) {
        match evt {
            PlayerEvent::NowPlaying(title, artist) => {
                self.state.player.status = PlayStatus::Playing;
                self.state.player.current_track = Some((title, artist));
            }
            PlayerEvent::Progress(pos, dur) => {
                self.state.player.position = pos;
                self.state.player.duration = dur;
            }
            PlayerEvent::Finished => {
                let last_video_id = self.state.player.current_video_id.clone();
                self.state.player.status = PlayStatus::Stopped;
                self.state.player.current_track = None;
                self.state.player.current_video_id = None;
                self.state.player.position = 0.0;
                self.state.player.duration = 0.0;
                self.cover_image = None;
                self.current_cover_video_id = None;
                self.play_next_in_queue();
                if self.state.auto_play && self.state.queue.is_empty() {
                    if let Some(ref vid) = last_video_id {
                        self.fetch_recommendations(vid);
                    }
                }
            }
            PlayerEvent::Error(e) => {
                self.state.status_message = Some(e);
            }
            PlayerEvent::StatusMessage(msg) => {
                self.state.status_message = Some(msg);
            }
        }
    }

    fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        player_cmd_tx: &mpsc::Sender<PlayerCommand>,
        ytm: &Arc<YtmClient>,
    ) {
        if key.code == KeyCode::Char('c') && key.modifiers == crossterm::event::KeyModifiers::CONTROL {
            self.should_quit = true;
            return;
        }

        if self.state.palette_visible {
            self.handle_palette_key(key, player_cmd_tx);
            return;
        }

        if self.state.resize_mode {
            match key.code {
                KeyCode::Esc => {
                    self.state.resize_mode = false;
                }
                KeyCode::Char('r') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                    self.state.resize_mode = false;
                }
                KeyCode::Left => {
                    let has_right_panel = self.state.queue_visible || self.state.lyrics_visible || self.state.help_visible;
                    if has_right_panel && self.state.focus == Focus::Queue {
                        self.state.right_panel_width = (self.state.right_panel_width + 1).clamp(10, 80);
                    } else {
                        self.state.sidebar_width = self.state.sidebar_width.saturating_sub(1).clamp(10, 80);
                    }
                }
                KeyCode::Right => {
                    let has_right_panel = self.state.queue_visible || self.state.lyrics_visible || self.state.help_visible;
                    if has_right_panel && self.state.focus == Focus::Queue {
                        self.state.right_panel_width = self.state.right_panel_width.saturating_sub(1).clamp(10, 80);
                    } else {
                        self.state.sidebar_width = (self.state.sidebar_width + 1).clamp(10, 80);
                    }
                }
                KeyCode::Up => {
                    self.state.right_panel_width = (self.state.right_panel_width + 1).clamp(10, 80);
                }
                KeyCode::Down => {
                    self.state.right_panel_width = self.state.right_panel_width.saturating_sub(1).clamp(10, 80);
                }
                _ => {}
            }
            return;
        }

        if self.state.focus == Focus::Search {
            self.handle_search_key(key, player_cmd_tx, ytm);
            return;
        }

        match key.code {
            KeyCode::Char('r') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                self.state.resize_mode = !self.state.resize_mode;
            }
            KeyCode::Char(':') => {
                self.open_command_palette();
            }
            KeyCode::Char('p') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                self.open_command_palette();
            }
            KeyCode::Char('c') if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                self.should_quit = true;
            }

            KeyCode::Char('R') | KeyCode::Char('r') if self.state.active_page == crate::state::app_state::ActivePage::Explore => {
                self.refresh_for_you(ytm);
            }
            KeyCode::Tab => {
                self.state.focus = match self.state.focus {
                    Focus::Sidebar => {
                        if self.state.active_page == crate::state::app_state::ActivePage::Search {
                            Focus::Search
                        } else {
                            if self.state.is_dual_box() {
                                self.state.search_tab = SearchTab::Songs;
                            }
                            Focus::Tracklist
                        }
                    }
                    Focus::Search => {
                        if self.state.tracks.is_empty() {
                            if self.state.queue_visible {
                                Focus::Queue
                            } else {
                                Focus::Sidebar
                            }
                        } else {
                            if self.state.is_dual_box() {
                                self.state.search_tab = SearchTab::Songs;
                            }
                            Focus::Tracklist
                        }
                    }
                    Focus::Tracklist => {
                        if self.state.is_dual_box() && self.state.search_tab == SearchTab::Songs {
                            self.state.search_tab = SearchTab::Videos;
                            Focus::Tracklist
                        } else if self.state.queue_visible {
                            Focus::Queue
                        } else {
                            Focus::Sidebar
                        }
                    }
                    Focus::Queue => Focus::Sidebar,
                };
            }
            KeyCode::BackTab => {
                self.state.focus = match self.state.focus {
                    Focus::Sidebar => {
                        if self.state.queue_visible {
                            Focus::Queue
                        } else {
                            if self.state.is_dual_box() {
                                self.state.search_tab = SearchTab::Videos;
                                Focus::Tracklist
                            } else if self.state.active_page == crate::state::app_state::ActivePage::Search {
                                Focus::Search
                            } else {
                                Focus::Tracklist
                            }
                        }
                    }
                    Focus::Search => Focus::Sidebar,
                    Focus::Tracklist => {
                        if self.state.is_dual_box() && self.state.search_tab == SearchTab::Videos {
                            self.state.search_tab = SearchTab::Songs;
                            Focus::Tracklist
                        } else if self.state.active_page == crate::state::app_state::ActivePage::Search {
                            Focus::Search
                        } else {
                            Focus::Sidebar
                        }
                    }
                    Focus::Queue => {
                        if self.state.is_dual_box() {
                            self.state.search_tab = SearchTab::Videos;
                        }
                        Focus::Tracklist
                    }
                };
            }
            KeyCode::Char('/') => {
                self.state.active_page = crate::state::app_state::ActivePage::Search;
                self.state.focus = Focus::Search;
            }
            KeyCode::Esc => {
                if self.state.focus == Focus::Search {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == crate::state::app_state::ActivePage::Library && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == crate::state::app_state::ActivePage::Search && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Search;
                }
                if self.state.active_page == crate::state::app_state::ActivePage::Explore && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.lyrics_visible {
                    self.state.lyrics_visible = false;
                }
                if self.state.help_visible {
                    self.state.help_visible = false;
                }
            }
            KeyCode::Backspace => {
                if self.state.active_page == crate::state::app_state::ActivePage::Library && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == crate::state::app_state::ActivePage::Explore && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
            }
            KeyCode::Left => {
                match self.state.focus {
                    Focus::Queue => {
                        self.state.focus = Focus::Tracklist;
                    }
                    Focus::Tracklist => {
                        self.state.focus = Focus::Sidebar;
                    }
                    Focus::Search => {
                        self.state.focus = Focus::Sidebar;
                    }
                    _ => {}
                }
            }
            KeyCode::Right => {
                match self.state.focus {
                    Focus::Sidebar => {
                        if self.state.active_page == crate::state::app_state::ActivePage::Search {
                            self.state.focus = Focus::Search;
                        } else {
                            self.state.focus = Focus::Tracklist;
                        }
                    }
                    Focus::Search => {
                        if !self.state.tracks.is_empty() {
                            self.state.focus = Focus::Tracklist;
                        }
                    }
                    Focus::Tracklist => {
                        if self.state.queue_visible {
                            self.state.focus = Focus::Queue;
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                match self.state.focus {
                    Focus::Sidebar => {
                        self.activate_sidebar_item(self.state.sidebar_index, ytm);
                        if self.state.active_page == crate::state::app_state::ActivePage::Library {
                            self.state.focus = Focus::Tracklist;
                        }
                    }
                    Focus::Tracklist => {
                        if self.state.active_page == crate::state::app_state::ActivePage::Explore {
                            use crate::state::app_state::ExploreSection;
                            match self.state.explore_section {
                                ExploreSection::ForYou => {
                                    let global_idx = self.state.active_track_global_index();
                                    if let Some(track) = self.state.explore_for_you.get(global_idx) {
                                        let track = Arc::clone(track);
                                        if global_idx + 1 < self.state.explore_for_you.len() {
                                            self.state.queue = self.state.explore_for_you[global_idx + 1..].to_vec();
                                        } else {
                                            self.state.queue.clear();
                                        }
                                        self.state.queue_index = 0;
                                        self.state.history_cursor = 0;
                                        self.play_track_item(&track, player_cmd_tx, true);
                                    }
                                }
                                ExploreSection::History => {
                                    let global_idx = self.state.active_track_global_index();
                                    if let Some(track) = self.state.history.get(global_idx) {
                                        let track = Arc::clone(track);
                                        self.state.queue.clear();
                                        self.state.queue_index = 0;
                                        self.state.history_cursor = 0;
                                        self.play_track_item(&track, player_cmd_tx, true);
                                    }
                                }
                                ExploreSection::TopArtists => {
                                    match self.state.search_tab {
                                        crate::state::app_state::SearchTab::Songs => {
                                            if let Some((artist_name, _)) = self.state.explore_top_artists.get(self.state.explore_artist_index) {
                                                let artist_name = artist_name.clone();
                                                self.select_artist(&artist_name, ytm);
                                            }
                                        }
                                        crate::state::app_state::SearchTab::Videos => {
                                            if let Some((channel_name, _)) = self.state.explore_top_channels.get(self.state.explore_channel_index) {
                                                let channel_name = channel_name.clone();
                                                self.select_artist(&channel_name, ytm);
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            self.play_selected_track(player_cmd_tx);
                        }
                    }
                    Focus::Queue => {
                        if self.state.queue_visible {
                            self.play_from_queue(player_cmd_tx);
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Up => {
                match self.state.focus {
                    Focus::Sidebar => {
                        if self.state.sidebar_index > 0 {
                            self.state.sidebar_index -= 1;
                        }
                    }
                    Focus::Tracklist => {
                        self.state.decrement_active_track_index();
                    }
                    Focus::Queue => {
                        if self.state.queue_visible {
                            if self.state.queue_index > 0 {
                                self.state.queue_index -= 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.state.focus {
                    Focus::Sidebar => {
                        let max = self.state.sidebar_items.len().saturating_sub(1);
                        if self.state.sidebar_index < max {
                            self.state.sidebar_index += 1;
                        }
                    }
                    Focus::Tracklist => {
                        self.state.increment_active_track_index();
                    }
                    Focus::Queue => {
                        if self.state.queue_visible {
                            let max = self.state.queue.len().saturating_sub(1);
                            if self.state.queue_index < max {
                                self.state.queue_index += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Char(' ') => {
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
            }
            KeyCode::Char('s') => {
                self.state.player.shuffle = !self.state.player.shuffle;
            }
            KeyCode::Char('r') => {
                self.state.player.repeat = match self.state.player.repeat {
                    RepeatMode::None => RepeatMode::All,
                    RepeatMode::All => RepeatMode::One,
                    RepeatMode::One => RepeatMode::None,
                };
            }
            KeyCode::Char('a') => {
                self.state.auto_play = !self.state.auto_play;
            }
            KeyCode::Char('q') => {
                self.toggle_queue();
            }
            KeyCode::Char('l') => {
                self.toggle_lyrics();
            }
            KeyCode::Char('?') => {
                self.toggle_help();
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_volume(1.0, player_cmd_tx);
            }
            KeyCode::Char('-') => {
                self.adjust_volume(-1.0, player_cmd_tx);
            }
            KeyCode::Char('n') => {
                if !self.state.queue.is_empty() {
                    let _ = player_cmd_tx.send(PlayerCommand::Stop);
                    self.play_next_in_queue();
                }
            }
            KeyCode::Char('p') => {
                if !self.state.queue.is_empty() {
                    let _ = player_cmd_tx.send(PlayerCommand::Stop);
                    self.play_previous_in_queue();
                }
            }
            KeyCode::Char('c') if key.modifiers == crossterm::event::KeyModifiers::NONE => {
                self.open_context_menu();
            }
            _ => {}
        }
    }
}

fn strip_inline_timestamps(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut temp = String::new();
            let mut found_end = false;
            while let Some(&next_c) = chars.peek() {
                if next_c == '>' {
                    chars.next();
                    found_end = true;
                    break;
                }
                temp.push(chars.next().unwrap());
            }
            if found_end {
                if temp.contains(':') && temp.chars().all(|ch| ch.is_ascii_digit() || ch == ':' || ch == '.' || ch == '_') {
                    continue;
                } else {
                    result.push('<');
                    result.push_str(&temp);
                    result.push('>');
                }
            } else {
                result.push('<');
                result.push_str(&temp);
            }
        } else {
            result.push(c);
        }
    }
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_lrc(lrc_text: &str) -> Vec<SyncedLine> {
    let mut lines = Vec::new();
    for line in lrc_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut text_start = 0;
        let mut timestamps = Vec::new();
        while let Some(end_idx) = line[text_start..].find(']') {
            let start_bracket = line[text_start..].find('[');
            if let Some(start_idx) = start_bracket {
                if start_idx < end_idx {
                    let ts_str = &line[text_start + start_idx + 1..text_start + end_idx];
                    if let Some(secs) = parse_lrc_timestamp(ts_str) {
                        timestamps.push(secs);
                    }
                    text_start = text_start + end_idx + 1;
                    continue;
                }
            }
            break;
        }
        let raw_text = line[text_start..].trim();
        let text = strip_inline_timestamps(raw_text);
        for ts in timestamps {
            lines.push(SyncedLine { timestamp: ts, text: text.clone() });
        }
    }
    lines.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn parse_lrc_timestamp(ts_str: &str) -> Option<f64> {
    crate::util::parse_time_string(ts_str)
}

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

