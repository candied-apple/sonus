use std::sync::Arc;

use sonus_core::api::client::YtmClient;
use crate::app::App;
use crate::state::app_state::{Focus, SidebarItem};

impl App {
    pub(crate) fn activate_sidebar_item(&mut self, clicked_idx: usize, ytm: &Arc<YtmClient>) {
        if clicked_idx >= self.state.sidebar_items.len() {
            return;
        }
        let item = &self.state.sidebar_items[clicked_idx];
        self.state.sidebar_index = clicked_idx;
        self.state.active_local_playlist_id = None;

        match item.playlist_id.as_deref() {
            Some("search") => {
                self.state.active_page = crate::state::app_state::ActivePage::Search;
                self.state.focus = Focus::Search;
            }
            Some("foryou") => {
                self.state.active_page = crate::state::app_state::ActivePage::Explore;
                self.state.explore_section = crate::state::app_state::ExploreSection::ForYou;
                self.state.is_search_results = false;
                self.state.tracks.clear();
                self.load_explore_data(ytm);
                self.state.focus = Focus::Tracklist;
            }
            Some("history") => {
                self.state.active_page = crate::state::app_state::ActivePage::Explore;
                self.state.explore_section = crate::state::app_state::ExploreSection::History;
                self.state.is_search_results = false;
                self.state.tracks.clear();
                self.load_explore_data(ytm);
                self.state.focus = Focus::Tracklist;
            }
            Some("topartists") => {
                self.state.active_page = crate::state::app_state::ActivePage::Explore;
                self.state.explore_section = crate::state::app_state::ExploreSection::TopArtists;
                self.state.is_search_results = false;
                self.state.tracks.clear();
                self.load_explore_data(ytm);
                self.state.focus = Focus::Tracklist;
            }
            _ => {
                if let Some(local_id) = item.local_playlist_id {
                    self.state.active_page = crate::state::app_state::ActivePage::Library;
                    self.state.active_local_playlist_id = Some(local_id);
                    self.state.view_title = format!("{}  Playlist", item.label);
                    if let Ok(tracks) = self.db.get_playlist_tracks(local_id) {
                        self.state.tracks = tracks.into_iter().map(Arc::new).collect();
                        self.state.update_search_results_cache();
                        self.state.track_index = 0;
                        self.state.track_offset = 0;
                        self.state.focus = Focus::Tracklist;
                    }
                }
            }
        }
    }

    pub(crate) fn load_playlists_to_sidebar(&mut self) {
        let mut items = vec![
            SidebarItem {
                label: "Search".to_string(),
                playlist_id: Some("search".to_string()),
                local_playlist_id: None,
            },
            SidebarItem {
                label: "For You".to_string(),
                playlist_id: Some("foryou".to_string()),
                local_playlist_id: None,
            },
            SidebarItem {
                label: "History".to_string(),
                playlist_id: Some("history".to_string()),
                local_playlist_id: None,
            },
            SidebarItem {
                label: "Top Artists".to_string(),
                playlist_id: Some("topartists".to_string()),
                local_playlist_id: None,
            },
        ];
        if let Ok(playlists) = self.db.get_playlists() {
            for (id, name) in playlists {
                items.push(SidebarItem {
                    label: name,
                    playlist_id: None,
                    local_playlist_id: Some(id),
                });
            }
        }
        self.state.sidebar_items = items;
        let max = self.state.sidebar_items.len().saturating_sub(1);
        if self.state.sidebar_index > max {
            self.state.sidebar_index = max;
        }
    }

    pub(crate) fn toggle_queue(&mut self) {
        self.state.queue_visible = !self.state.queue_visible;
        self.state.needs_clear = true;
        if self.state.queue_visible {
            self.state.lyrics_visible = false;
            self.state.help_visible = false;
        } else if self.state.focus == Focus::Queue {
            self.state.focus = Focus::Tracklist;
        }
    }

    pub(crate) fn toggle_lyrics(&mut self) {
        self.state.lyrics_visible = !self.state.lyrics_visible;
        self.state.needs_clear = true;
        if self.state.lyrics_visible {
            self.state.queue_visible = false;
            self.state.help_visible = false;
            self.fetch_lyrics_for_current_track();
        } else if self.state.focus == Focus::Queue {
            self.state.focus = Focus::Tracklist;
        }
    }

    pub(crate) fn toggle_help(&mut self) {
        self.state.help_visible = !self.state.help_visible;
        self.state.needs_clear = true;
        if self.state.help_visible {
            self.state.queue_visible = false;
            self.state.lyrics_visible = false;
        } else if self.state.focus == Focus::Queue {
            self.state.focus = Focus::Tracklist;
        }
    }
}
