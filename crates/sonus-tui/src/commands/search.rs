use std::sync::Arc;

use tokio::sync::oneshot;

use sonus_core::api::client::YtmClient;
use crate::app::{ApiResult, App};

impl App {
    pub(crate) fn handle_search_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        _player_cmd_tx: &std::sync::mpsc::Sender<sonus_core::player::PlayerCommand>,
        ytm: &Arc<YtmClient>,
    ) {
        match key.code {
            crossterm::event::KeyCode::Esc => {
                self.state.search_query.clear();
                self.state.is_search_results = false;
                self.state.tracks.clear();
                self.state.focus = crate::state::app_state::Focus::Sidebar;
            }
            crossterm::event::KeyCode::Enter => {
                let q = self.state.search_query.clone();
                if !q.is_empty() && self.pending_api.is_none() {
                    self.state.view_title = "Search Results".into();
                    self.state.search_tab = crate::state::app_state::SearchTab::Songs;
                    self.state.song_index = 0;
                    self.state.song_offset = 0;
                    self.state.video_index = 0;
                    self.state.video_offset = 0;
                    let ytm = Arc::clone(ytm);
                    let (tx, rx) = oneshot::channel();
                    self.pending_api = Some(rx);
                    self.state.status_message = Some("Searching...".into());
                    self.state.focus = crate::state::app_state::Focus::Tracklist;
                    tokio::spawn(async move {
                        match ytm.search_all(&q).await {
                            Ok(tracks) => { let _ = tx.send(ApiResult::Tracks(tracks)); }
                            Err(e) => { let _ = tx.send(ApiResult::Error(e)); }
                        }
                    });
                }
            }
            crossterm::event::KeyCode::Backspace => {
                self.state.search_query.pop();
            }
            crossterm::event::KeyCode::Char(c) => {
                self.state.search_query.push(c);
            }
            _ => {}
        }
    }
}
