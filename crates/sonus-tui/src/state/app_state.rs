use std::sync::Arc;
pub use sonus_core::types::{PlayStatus, RepeatMode, TrackCategory, TrackItem, PlayerState, SyncedLine};

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Search,
    Sidebar,
    Tracklist,
    Queue,
}

#[derive(Debug, Clone)]
pub struct SidebarItem {
    pub label: String,
    pub playlist_id: Option<String>,
    pub local_playlist_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchTab {
    Songs,
    Videos,
}



#[derive(Debug, Clone)]
pub struct SpotifyImportState {
    pub playlist_name: String,
    pub completed: usize,
    pub total_tracks: usize,
    pub current_track_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExploreSection {
    ForYou,
    History,
    TopArtists,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePage {
    Library,
    Search,
    Explore,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_page: ActivePage,
    pub focus: Focus,
    pub search_query: String,
    pub sidebar_items: Vec<SidebarItem>,
    pub sidebar_index: usize,
    pub sidebar_offset: usize,
    pub track_album: Option<(String, String)>,
    pub tracks: Vec<Arc<TrackItem>>,
    pub track_index: usize,
    pub track_offset: usize,
    pub player: PlayerState,
    pub queue: Vec<Arc<TrackItem>>,
    pub queue_index: usize,
    pub queue_visible: bool,
    pub history: Vec<Arc<TrackItem>>,
    pub history_index: usize,
    pub history_cursor: usize,
    pub lyrics_visible: bool,
    pub help_visible: bool,
    pub lyrics_text: Option<String>,
    pub synced_lyrics: Option<Vec<SyncedLine>>,
    pub status_message: Option<String>,
    pub api_ready: bool,
    pub view_title: String,
    pub active_local_playlist_id: Option<i32>,

    pub palette_visible: bool,
    pub palette_mode: crate::state::command_palette::PaletteMode,
    pub palette_input: String,
    pub palette_selected: usize,
    pub palette_items: Vec<String>,
    pub palette_track: Option<Arc<TrackItem>>,
    pub sidebar_width: u16,
    pub right_panel_width: u16,
    pub resize_mode: bool,
    pub dragging_sidebar_border: bool,
    pub dragging_right_panel_border: bool,
    pub spotify_import: Option<SpotifyImportState>,
    pub cached_playlists: Option<Vec<(i32, String)>>,
    pub search_tab: SearchTab,
    pub song_index: usize,
    pub song_offset: usize,
    pub video_index: usize,
    pub video_offset: usize,
    pub terminal_area: ratatui::layout::Rect,
    pub needs_clear: bool,
    pub is_search_results: bool,
    pub auto_play: bool,
    pub explore_section: ExploreSection,
    pub explore_for_you: Vec<Arc<TrackItem>>,
    pub explore_top_artists: Vec<(String, usize)>,
    pub explore_top_channels: Vec<(String, usize)>,
    pub explore_artist_index: usize,
    pub explore_channel_index: usize,
    pub explore_loaded: bool,
    pub new_version_available: Option<String>,
}

impl AppState {
    pub fn is_search_results(&self) -> bool {
        self.is_search_results
    }

    pub fn is_dual_box(&self) -> bool {
        self.active_page == ActivePage::Search
            || self.is_search_results()
            || (self.active_page == ActivePage::Explore
                && (self.explore_section == ExploreSection::ForYou
                    || self.explore_section == ExploreSection::History
                    || self.explore_section == ExploreSection::TopArtists))
    }

    pub fn update_search_results_cache(&mut self) {
        self.is_search_results = self.tracks.iter().any(|t| t.category == TrackCategory::Song)
            && self.tracks.iter().any(|t| t.category == TrackCategory::Video);
    }

    pub fn active_track_global_index(&self) -> usize {
        if self.active_page == ActivePage::Explore && !self.is_dual_box() {
            if self.explore_section == ExploreSection::TopArtists {
                return self.explore_artist_index;
            } else {
                return self.track_index;
            }
        }
        if !self.is_dual_box() {
            return self.track_index;
        }

        let source_list = match self.active_page {
            ActivePage::Library => &self.tracks,
            ActivePage::Search => &self.tracks,
            ActivePage::Explore => match self.explore_section {
                ExploreSection::ForYou => &self.explore_for_you,
                ExploreSection::History => &self.history,
                ExploreSection::TopArtists => &self.tracks,
            },
        };
        let source_list = if self.is_search_results() { &self.tracks } else { source_list };

        match self.search_tab {
            SearchTab::Songs => {
                let mut song_cnt = 0;
                for (i, t) in source_list.iter().enumerate() {
                    if t.category == TrackCategory::Song {
                        if song_cnt == self.song_index {
                            return i;
                        }
                        song_cnt += 1;
                    }
                }
                0
            }
            SearchTab::Videos => {
                let mut video_cnt = 0;
                for (i, t) in source_list.iter().enumerate() {
                    if t.category == TrackCategory::Video {
                        if video_cnt == self.video_index {
                            return i;
                        }
                        video_cnt += 1;
                    }
                }
                0
            }
        }
    }

    pub fn active_track_count(&self) -> usize {
        if self.active_page == ActivePage::Explore && self.explore_section == ExploreSection::TopArtists {
            return match self.search_tab {
                SearchTab::Songs => self.explore_top_artists.len(),
                SearchTab::Videos => self.explore_top_channels.len(),
            };
        }
        if self.active_page == ActivePage::Explore && !self.is_dual_box() {
            return match self.explore_section {
                ExploreSection::TopArtists => self.explore_top_artists.len(),
                _ => 0,
            };
        }
        let source_list = match self.active_page {
            ActivePage::Library => &self.tracks,
            ActivePage::Search => &self.tracks,
            ActivePage::Explore => match self.explore_section {
                ExploreSection::ForYou => &self.explore_for_you,
                ExploreSection::History => &self.history,
                ExploreSection::TopArtists => &self.tracks,
            },
        };
        let source_list = if self.is_search_results() { &self.tracks } else { source_list };

        if !self.is_dual_box() {
            return source_list.len();
        }

        match self.search_tab {
            SearchTab::Songs => source_list.iter().filter(|t| t.category == TrackCategory::Song).count(),
            SearchTab::Videos => source_list.iter().filter(|t| t.category == TrackCategory::Video).count(),
        }
    }

    pub fn decrement_active_track_index(&mut self) {
        if self.active_page == ActivePage::Explore && self.explore_section == ExploreSection::TopArtists {
            match self.search_tab {
                SearchTab::Songs => {
                    if self.explore_artist_index > 0 { self.explore_artist_index -= 1; }
                }
                SearchTab::Videos => {
                    if self.explore_channel_index > 0 { self.explore_channel_index -= 1; }
                }
            }
            return;
        }
        if self.active_page == ActivePage::Explore && !self.is_dual_box() {
            match self.explore_section {
                ExploreSection::TopArtists => {
                    if self.explore_artist_index > 0 { self.explore_artist_index -= 1; }
                }
                _ => {
                    if self.track_index > 0 { self.track_index -= 1; }
                }
            }
            return;
        }
        if !self.is_dual_box() {
            if self.track_index > 0 { self.track_index -= 1; }
            return;
        }
        match self.search_tab {
            SearchTab::Songs => { if self.song_index > 0 { self.song_index -= 1; } }
            SearchTab::Videos => { if self.video_index > 0 { self.video_index -= 1; } }
        }
    }

    pub fn increment_active_track_index(&mut self) {
        let max = self.active_track_count().saturating_sub(1);
        if self.active_page == ActivePage::Explore && self.explore_section == ExploreSection::TopArtists {
            match self.search_tab {
                SearchTab::Songs => {
                    if self.explore_artist_index < max { self.explore_artist_index += 1; }
                }
                SearchTab::Videos => {
                    if self.explore_channel_index < max { self.explore_channel_index += 1; }
                }
            }
            return;
        }
        if self.active_page == ActivePage::Explore && !self.is_dual_box() {
            match self.explore_section {
                ExploreSection::TopArtists => {
                    if self.explore_artist_index < max { self.explore_artist_index += 1; }
                }
                _ => {
                    if self.track_index < max { self.track_index += 1; }
                }
            }
            return;
        }
        if !self.is_dual_box() {
            if self.track_index < max { self.track_index += 1; }
            return;
        }
        match self.search_tab {
            SearchTab::Songs => { if self.song_index < max { self.song_index += 1; } }
            SearchTab::Videos => { if self.video_index < max { self.video_index += 1; } }
        }
    }

    pub fn active_track(&self) -> Option<Arc<TrackItem>> {
        let source_list = match self.active_page {
            ActivePage::Library => &self.tracks,
            ActivePage::Search => &self.tracks,
            ActivePage::Explore => match self.explore_section {
                ExploreSection::ForYou => &self.explore_for_you,
                ExploreSection::History => &self.history,
                ExploreSection::TopArtists => &self.tracks,
            },
        };
        let source_list = if self.is_search_results() { &self.tracks } else { source_list };
        let idx = self.active_track_global_index();
        source_list.get(idx).cloned()
    }

    pub fn active_track_list(&self) -> Vec<Arc<TrackItem>> {
        let source_list = match self.active_page {
            ActivePage::Library => &self.tracks,
            ActivePage::Search => &self.tracks,
            ActivePage::Explore => match self.explore_section {
                ExploreSection::ForYou => &self.explore_for_you,
                ExploreSection::History => &self.history,
                ExploreSection::TopArtists => &self.tracks,
            },
        };
        let source_list = if self.is_search_results() { &self.tracks } else { source_list };

        if !self.is_dual_box() {
            return source_list.clone();
        }

        match self.search_tab {
            SearchTab::Songs => source_list.iter().filter(|t| t.category == TrackCategory::Song).cloned().collect(),
            SearchTab::Videos => source_list.iter().filter(|t| t.category == TrackCategory::Video).cloned().collect(),
        }
    }

    pub fn active_local_index(&self) -> usize {
        if self.active_page == ActivePage::Explore && self.explore_section == ExploreSection::TopArtists {
            return match self.search_tab {
                SearchTab::Songs => self.explore_artist_index,
                SearchTab::Videos => self.explore_channel_index,
            };
        }
        if !self.is_dual_box() {
            return self.track_index;
        }
        match self.search_tab {
            SearchTab::Songs => self.song_index,
            SearchTab::Videos => self.video_index,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_page: ActivePage::Library,
            focus: Focus::Sidebar,
            search_query: String::new(),
            sidebar_items: vec![],
            sidebar_index: 0,
            sidebar_offset: 0,
            track_album: None,
            tracks: vec![],
            track_index: 0,
            track_offset: 0,
            player: PlayerState::default(),
            queue: vec![],
            queue_index: 0,
            queue_visible: false,
            history: vec![],
            history_index: 0,
            history_cursor: 0,
            lyrics_visible: false,
            help_visible: false,
            lyrics_text: None,
            synced_lyrics: None,
            status_message: None,
            api_ready: false,
            view_title: String::new(),
            active_local_playlist_id: None,

            palette_visible: false,
            palette_mode: crate::state::command_palette::PaletteMode::CommandSelection,
            palette_input: String::new(),
            palette_selected: 0,
            palette_items: vec![],
            palette_track: None,
            sidebar_width: 30,
            right_panel_width: 30,
            resize_mode: false,
            dragging_sidebar_border: false,
            dragging_right_panel_border: false,
            spotify_import: None,
            cached_playlists: None,
            search_tab: SearchTab::Songs,
            song_index: 0,
            song_offset: 0,
            video_index: 0,
            video_offset: 0,
            terminal_area: ratatui::layout::Rect::default(),
            needs_clear: false,
            is_search_results: false,
            auto_play: true,
            explore_section: ExploreSection::ForYou,
            explore_for_you: vec![],
            explore_top_artists: vec![],
            explore_top_channels: vec![],
            explore_artist_index: 0,
            explore_channel_index: 0,
            explore_loaded: false,
            new_version_available: None,
        }
    }
}
