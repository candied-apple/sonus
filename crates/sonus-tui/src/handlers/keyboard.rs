use std::sync::mpsc;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::App;
use sonus_core::api::YtmClient;
use sonus_core::player::PlayerCommand;
use crate::state::app_state::{Focus, SearchTab, ActivePage, ExploreSection};

impl App {
    pub(crate) fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        player_cmd_tx: &mpsc::Sender<PlayerCommand>,
        ytm: &Arc<YtmClient>,
    ) {
        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
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
                KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
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
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
                self.state.resize_mode = !self.state.resize_mode;
            }
            KeyCode::Char(':') => {
                self.open_command_palette();
            }
            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                self.open_command_palette();
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.should_quit = true;
            }

            KeyCode::Char('R') | KeyCode::Char('r') if self.state.active_page == ActivePage::Explore => {
                self.refresh_for_you(ytm);
            }
            KeyCode::Tab => {
                self.state.focus = match self.state.focus {
                    Focus::Sidebar => {
                        if self.state.active_page == ActivePage::Search {
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
                            } else if self.state.active_page == ActivePage::Search {
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
                        } else if self.state.active_page == ActivePage::Search {
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
                self.state.active_page = ActivePage::Search;
                self.state.focus = Focus::Search;
            }
            KeyCode::Esc => {
                if self.state.focus == Focus::Search {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == ActivePage::Library && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == ActivePage::Search && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Search;
                }
                if self.state.active_page == ActivePage::Explore && self.state.focus == Focus::Tracklist {
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
                if self.state.active_page == ActivePage::Library && self.state.focus == Focus::Tracklist {
                    self.state.focus = Focus::Sidebar;
                }
                if self.state.active_page == ActivePage::Explore && self.state.focus == Focus::Tracklist {
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
                        if self.state.active_page == ActivePage::Search {
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
                        if self.state.active_page == ActivePage::Library {
                            self.state.focus = Focus::Tracklist;
                        }
                    }
                    Focus::Tracklist => {
                        if self.state.active_page == ActivePage::Explore {
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
                                        SearchTab::Songs => {
                                            if let Some((artist_name, _)) = self.state.explore_top_artists.get(self.state.explore_artist_index) {
                                                let artist_name = artist_name.clone();
                                                self.select_artist(&artist_name, ytm);
                                            }
                                        }
                                        SearchTab::Videos => {
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
                self.toggle_play_pause(player_cmd_tx);
            }
            KeyCode::Char('s') => {
                self.toggle_shuffle();
            }
            KeyCode::Char('r') => {
                self.cycle_repeat_mode();
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
            KeyCode::Char('c') if key.modifiers == KeyModifiers::NONE => {
                self.open_context_menu();
            }
            _ => {}
        }
    }
}
