use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use tokio::sync::oneshot;

use sonus_core::api::YtmClient;
use crate::handlers::import::ImportProgress;
use crate::ui;
use crate::lrc::parse_lrc;

use sonus_core::db::Db;
use sonus_core::player::{PlayerCommand, PlayerEvent};
use crate::state::app_state::{
    AppState, Focus,
};
use sonus_core::types::{PlayStatus, TrackItem, TrackCategory};


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
    pub(crate) mpris_cmd_rx: Option<std::sync::mpsc::Receiver<sonus_core::mpris::MprisCommand>>,
    pub(crate) mpris_signal_tx: Option<tokio::sync::mpsc::UnboundedSender<sonus_core::mpris::MprisSignal>>,
    pub(crate) mpris_state: Option<std::sync::Arc<std::sync::RwLock<sonus_core::mpris::MprisState>>>,
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
            mpris_cmd_rx: None,
            mpris_signal_tx: None,
            mpris_state: None,
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
            if let Ok(tag) = sonus_core::api::check_latest_release().await {
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

        // Initialize cross-platform media controls
        let (mpris_cmd_tx, mpris_cmd_rx) = std::sync::mpsc::channel();
        let (mpris_signal_tx, mpris_signal_rx) = tokio::sync::mpsc::unbounded_channel();
        let mpris_state = std::sync::Arc::new(std::sync::RwLock::new(sonus_core::mpris::MprisState::new()));

        self.mpris_cmd_rx = Some(mpris_cmd_rx);
        self.mpris_signal_tx = Some(mpris_signal_tx);
        self.mpris_state = Some(mpris_state.clone());

        sonus_core::mpris::spawn(mpris_cmd_tx, mpris_state, mpris_signal_rx);
        self.update_mpris_state();

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

            let mut mpris_cmds = Vec::new();
            if let Some(ref rx) = self.mpris_cmd_rx {
                while let Ok(cmd) = rx.try_recv() {
                    mpris_cmds.push(cmd);
                }
            }
            for cmd in mpris_cmds {
                self.handle_mpris_command(cmd);
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
                        crate::cache::save_for_you_cache(&final_tracks);
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

        if let Some(ref tx) = self.mpris_signal_tx {
            let _ = tx.send(sonus_core::mpris::MprisSignal::Shutdown);
        }

        Ok(())
    }
}

