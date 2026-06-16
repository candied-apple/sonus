use std::sync::mpsc;
use std::sync::Arc;

use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use sonus_core::api::client::YtmClient;
use crate::app::App;
use sonus_core::player::PlayerCommand;
use crate::state::app_state::{Focus, PlayStatus, RepeatMode, SearchTab};


impl App {
    pub(crate) fn handle_mouse(
        &mut self,
        mouse: MouseEvent,
        player_cmd_tx: &mpsc::Sender<PlayerCommand>,
        ytm: &Arc<YtmClient>,
    ) -> bool {
        let area = self.state.terminal_area;
        if area.width == 0 || area.height == 0 {
            return false;
        }
        let chunks = crate::ui::layout::calculate_layout(area, &self.state);

        let cx = mouse.column;
        let cy = mouse.row;

        match mouse.kind {
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                self.state.dragging_sidebar_border = false;
                self.state.dragging_right_panel_border = false;
                return true;
            }
            MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                if self.state.dragging_sidebar_border {
                    let new_width = cx.saturating_sub(chunks.sidebar.x);
                    self.state.sidebar_width = new_width.clamp(10, 80);
                    return true;
                } else if self.state.dragging_right_panel_border {
                    let right_edge = chunks.queue.x + chunks.queue.width;
                    let new_width = right_edge.saturating_sub(cx);
                    self.state.right_panel_width = new_width.clamp(10, 80);
                    return true;
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
                if rect_contains(chunks.sidebar, cx, cy) {
                    let inner = get_inner_rect(chunks.sidebar);
                    if rect_contains(inner, cx, cy) {
                        let click_y = (cy - inner.y) as usize;
                        let line_idx = self.state.sidebar_offset + click_y;
                        let clicked_idx = if line_idx < 4 {
                            Some(line_idx)
                        } else if line_idx > 4 {
                            Some(line_idx - 1)
                        } else {
                            None
                        };
                        if let Some(clicked_idx) = clicked_idx {
                            if clicked_idx < self.state.sidebar_items.len() {
                                self.state.focus = Focus::Sidebar;
                                self.activate_sidebar_item(clicked_idx, ytm);
                                self.open_context_menu();
                            }
                        }
                    }
                } else if rect_contains(chunks.tracklist, cx, cy) {
                    if self.state.active_page == crate::state::app_state::ActivePage::Search && cy < chunks.tracklist.y + 3 {
                        self.state.focus = Focus::Search;
                        return true;
                    }
                    self.state.focus = Focus::Tracklist;
                    let results_area = if self.state.active_page == crate::state::app_state::ActivePage::Search {
                        Rect {
                            x: chunks.tracklist.x,
                            y: chunks.tracklist.y + 3,
                            width: chunks.tracklist.width,
                            height: chunks.tracklist.height.saturating_sub(3),
                        }
                    } else {
                        chunks.tracklist
                    };

                    if self.state.is_dual_box() {
                        let half_h = results_area.height / 2;
                        if cy < results_area.y + half_h {
                            let box_area = Rect { x: results_area.x, y: results_area.y, width: results_area.width, height: half_h };
                            if let Some(local_idx) = tracklist_click_index(box_area, cy, self.state.song_offset) {
                                self.state.search_tab = SearchTab::Songs;
                                self.state.song_index = local_idx;
                                self.open_context_menu();
                            }
                        } else {
                            let box_area = Rect { x: results_area.x, y: results_area.y + half_h, width: results_area.width, height: results_area.height.saturating_sub(half_h) };
                            if let Some(local_idx) = tracklist_click_index(box_area, cy, self.state.video_offset) {
                                self.state.search_tab = SearchTab::Videos;
                                self.state.video_index = local_idx;
                                self.open_context_menu();
                            }
                        }
                    } else {
                        let inner = get_inner_rect(results_area);
                        if rect_contains(inner, cx, cy) {
                            let mut content_offset = 2;
                            if self.state.track_album.is_some() {
                                content_offset += 2;
                            }
                            let click_y = (cy - inner.y) as i32 - content_offset;
                            if click_y >= 0 {
                                let clicked_idx = self.state.track_offset + click_y as usize;
                                if clicked_idx < self.state.tracks.len() {
                                    self.state.track_index = clicked_idx;
                                    self.open_context_menu();
                                }
                            }
                        }
                    }
                } else if self.state.queue_visible && rect_contains(chunks.queue, cx, cy) {
                    self.state.focus = Focus::Queue;
                    let inner = get_inner_rect(chunks.queue);
                    if rect_contains(inner, cx, cy) {
                        let click_y = (cy - inner.y) as usize;
                        let mut offset = 0usize;
                        let visible = inner.height as usize;
                        crate::ui::components::ensure_scroll(self.state.queue_index, &mut offset, visible);
                        let clicked_idx = offset + click_y;
                        if clicked_idx < self.state.queue.len() {
                            self.state.queue_index = clicked_idx;
                            self.open_context_menu();
                        }
                    }
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let on_right_border = chunks.queue.width > 0 && (cx >= chunks.queue.x && cx <= chunks.queue.x + 1);

                if on_right_border {
                    self.state.dragging_right_panel_border = true;
                    return true;
                }

                // Header click
                if rect_contains(chunks.header, cx, cy) {
                    let header_parts = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Min(10), Constraint::Length(60)])
                        .split(chunks.header);

                    if rect_contains(header_parts[1], cx, cy) {
                        let right_boundary = header_parts[1].x + header_parts[1].width;
                        let use_nerd = crate::config::use_nerd_font();
                        let (q_width, l_width, h_width) = if use_nerd {
                            (9, 10, 3)
                        } else {
                            (7, 8, 3)
                        };
                        let h_start = right_boundary.saturating_sub(h_width);
                        let l_end = h_start.saturating_sub(2);
                        let l_start = l_end.saturating_sub(l_width);
                        let q_end = l_start.saturating_sub(2);
                        let q_start = q_end.saturating_sub(q_width);

                        if cx >= h_start && cx < right_boundary {
                            self.toggle_help();
                        } else if cx >= l_start && cx < l_end {
                            self.toggle_lyrics();
                        } else if cx >= q_start && cx < q_end {
                            self.toggle_queue();
                        }
                    }
                }
                // Sidebar click
                else if rect_contains(chunks.sidebar, cx, cy) {
                    let inner = get_inner_rect(chunks.sidebar);
                    if rect_contains(inner, cx, cy) {
                        let click_y = (cy - inner.y) as usize;
                        let line_idx = self.state.sidebar_offset + click_y;
                        let clicked_idx = if line_idx < 4 {
                            Some(line_idx)
                        } else if line_idx > 4 {
                            Some(line_idx - 1)
                        } else {
                            None
                        };
                        if let Some(clicked_idx) = clicked_idx {
                            if clicked_idx < self.state.sidebar_items.len() {
                                self.state.focus = Focus::Sidebar;
                                self.activate_sidebar_item(clicked_idx, ytm);
                            }
                        }
                    }
                }
                // Tracklist click
                else if rect_contains(chunks.tracklist, cx, cy) {
                    if self.state.active_page == crate::state::app_state::ActivePage::Search && cy < chunks.tracklist.y + 3 {
                        self.state.focus = Focus::Search;
                        return true;
                    }
                    let results_area = if self.state.active_page == crate::state::app_state::ActivePage::Search {
                        Rect {
                            x: chunks.tracklist.x,
                            y: chunks.tracklist.y + 3,
                            width: chunks.tracklist.width,
                            height: chunks.tracklist.height.saturating_sub(3),
                        }
                    } else {
                        chunks.tracklist
                    };

                    self.state.focus = Focus::Tracklist;
                    if self.state.is_dual_box() {
                        let half_h = results_area.height / 2;

                        if self.state.active_page == crate::state::app_state::ActivePage::Explore
                            && self.state.explore_section == crate::state::app_state::ExploreSection::TopArtists
                        {
                            if cy < results_area.y + half_h {
                                let box_area = Rect { x: results_area.x, y: results_area.y, width: results_area.width, height: half_h };
                                let inner = get_inner_rect(box_area);
                                if cy >= inner.y && cy < inner.y + inner.height {
                                    let click_y = (cy - inner.y) as usize;
                                    let height = inner.height as usize;
                                    let start = self.state.explore_artist_index.saturating_sub(height / 2);
                                    let end = (start + height).min(self.state.explore_top_artists.len());
                                    let start = end.saturating_sub(height).min(start);
                                    let clicked_idx = start + click_y;
                                    if clicked_idx < self.state.explore_top_artists.len() {
                                        self.state.search_tab = SearchTab::Songs;
                                        self.state.explore_artist_index = clicked_idx;
                                        let (artist_name, _) = self.state.explore_top_artists[clicked_idx].clone();
                                        self.select_artist(&artist_name, ytm);
                                    }
                                }
                            } else {
                                let box_area = Rect { x: results_area.x, y: results_area.y + half_h, width: results_area.width, height: results_area.height.saturating_sub(half_h) };
                                let inner = get_inner_rect(box_area);
                                if cy >= inner.y && cy < inner.y + inner.height {
                                    let click_y = (cy - inner.y) as usize;
                                    let height = inner.height as usize;
                                    let start = self.state.explore_channel_index.saturating_sub(height / 2);
                                    let end = (start + height).min(self.state.explore_top_channels.len());
                                    let start = end.saturating_sub(height).min(start);
                                    let clicked_idx = start + click_y;
                                    if clicked_idx < self.state.explore_top_channels.len() {
                                        self.state.search_tab = SearchTab::Videos;
                                        self.state.explore_channel_index = clicked_idx;
                                        let (channel_name, _) = self.state.explore_top_channels[clicked_idx].clone();
                                        self.select_artist(&channel_name, ytm);
                                    }
                                }
                            }
                        } else {
                            if cy < results_area.y + half_h {
                                let box_area = Rect { x: results_area.x, y: results_area.y, width: results_area.width, height: half_h };
                                if let Some(local_idx) = tracklist_click_index(box_area, cy, self.state.song_offset) {
                                    self.state.search_tab = SearchTab::Songs;
                                    self.state.song_index = local_idx;
                                    self.play_selected_track(player_cmd_tx);
                                }
                            } else {
                                let box_area = Rect { x: results_area.x, y: results_area.y + half_h, width: results_area.width, height: results_area.height.saturating_sub(half_h) };
                                if let Some(local_idx) = tracklist_click_index(box_area, cy, self.state.video_offset) {
                                    self.state.search_tab = SearchTab::Videos;
                                    self.state.video_index = local_idx;
                                    self.play_selected_track(player_cmd_tx);
                                }
                            }
                        }
                    } else {
                        let inner = get_inner_rect(results_area);
                        if rect_contains(inner, cx, cy) {
                            let mut content_offset = 2;
                            if self.state.track_album.is_some() {
                                content_offset += 2;
                            }
                            let click_y = (cy - inner.y) as i32 - content_offset;
                            if click_y >= 0 {
                                let clicked_idx = self.state.track_offset + click_y as usize;
                                if clicked_idx < self.state.tracks.len() {
                                    self.state.track_index = clicked_idx;
                                    self.play_selected_track(player_cmd_tx);
                                }
                            }
                        }
                    }
                }
                // Queue/History click
                else if self.state.queue_visible && rect_contains(chunks.queue, cx, cy) {
                    self.state.focus = Focus::Queue;
                    let inner = get_inner_rect(chunks.queue);
                    if rect_contains(inner, cx, cy) {
                        let click_y = (cy - inner.y) as usize;
                        let mut offset = 0usize;
                        let visible = inner.height as usize;
                        crate::ui::components::ensure_scroll(self.state.queue_index, &mut offset, visible);
                        let clicked_idx = offset + click_y;
                        if clicked_idx < self.state.queue.len() {
                            self.state.queue_index = clicked_idx;
                            self.play_from_queue(player_cmd_tx);
                        }
                    }
                }
                // Footer (playback control icons) click
                else if rect_contains(chunks.footer, cx, cy) {
                    let inner = get_inner_rect(chunks.footer);
                    let footer_parts = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(1), // Left padding
                            Constraint::Length(6), // Image
                            Constraint::Length(1), // Spacer
                            Constraint::Min(1),    // Text and controls
                        ])
                        .split(inner);

                    if rect_contains(footer_parts[3], cx, cy) {
                        let rows = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Min(0),
                            ])
                            .split(footer_parts[3]);

                        if rect_contains(rows[2], cx, cy) {
                            let x = rows[2].x;
                            let offset = cx.saturating_sub(x) as usize;

                            let use_nerd = crate::config::use_nerd_font();
                            let is_playing = self.state.player.status == PlayStatus::Playing;
                            let shuffle_state = if self.state.player.shuffle { "on" } else { "off" };
                            let repeat_state = match self.state.player.repeat {
                                RepeatMode::None => "off",
                                RepeatMode::One => "one",
                                RepeatMode::All => "all",
                            };
                            let auto_state = if self.state.auto_play { "on" } else { "off" };

                            let (prev_range, play_range, next_range, shuffle_range, repeat_range, auto_range) = if use_nerd {
                                let prev_start = 1;
                                let prev_end = prev_start + 1;

                                let play_start = prev_end + 2;
                                let play_end = play_start + 1;

                                let next_start = play_end + 2;
                                let next_end = next_start + 1;

                                let shuffle_start = next_end + 3;
                                let shuffle_end = shuffle_start + 2 + shuffle_state.len();

                                let repeat_start = shuffle_end + 3;
                                let repeat_end = repeat_start + 2 + repeat_state.len();

                                let auto_start = repeat_end + 3;
                                let auto_end = auto_start + 2 + auto_state.len();

                                (
                                    prev_start..prev_end,
                                    play_start..play_end,
                                    next_start..next_end,
                                    shuffle_start..shuffle_end,
                                    repeat_start..repeat_end,
                                    auto_start..auto_end,
                                )
                            } else {
                                let play_icon_len = if is_playing { 2 } else { 1 };
                                let prev_start = 1;
                                let prev_end = prev_start + 2;

                                let play_start = prev_end + 2;
                                let play_end = play_start + play_icon_len;

                                let next_start = play_end + 2;
                                let next_end = next_start + 2;

                                let shuffle_start = next_end + 3;
                                let shuffle_end = shuffle_start + 9 + shuffle_state.len();

                                let repeat_start = shuffle_end + 3;
                                let repeat_end = repeat_start + 8 + repeat_state.len();

                                let auto_start = repeat_end + 3;
                                let auto_end = auto_start + 6 + auto_state.len();

                                (
                                    prev_start..prev_end,
                                    play_start..play_end,
                                    next_start..next_end,
                                    shuffle_start..shuffle_end,
                                    repeat_start..repeat_end,
                                    auto_start..auto_end,
                                )
                            };

                            if prev_range.contains(&offset) {
                                if !self.state.queue.is_empty() {
                                    let _ = player_cmd_tx.send(PlayerCommand::Stop);
                                    self.play_previous_in_queue();
                                }
                            } else if play_range.contains(&offset) {
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
                            } else if next_range.contains(&offset) {
                                if !self.state.queue.is_empty() {
                                    let _ = player_cmd_tx.send(PlayerCommand::Stop);
                                    self.play_next_in_queue();
                                }
                            } else if shuffle_range.contains(&offset) {
                                self.state.player.shuffle = !self.state.player.shuffle;
                            } else if repeat_range.contains(&offset) {
                                self.state.player.repeat = match self.state.player.repeat {
                                    RepeatMode::None => RepeatMode::All,
                                    RepeatMode::All => RepeatMode::One,
                                    RepeatMode::One => RepeatMode::None,
                                };
                            } else if auto_range.contains(&offset) {
                                self.state.auto_play = !self.state.auto_play;
                            }
                        } else if rect_contains(rows[1], cx, cy) {
                            let duration = self.state.player.duration;
                            if duration > 0.0 {
                                let cols = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Min(1),
                                        Constraint::Length(2),
                                    ])
                                    .split(rows[1]);

                                if cx >= cols[0].x && cx < cols[0].x + cols[0].width {
                                    let click_x = cx.saturating_sub(cols[0].x) as f64;
                                    let ratio = (click_x / cols[0].width as f64).clamp(0.0, 1.0);
                                    let target_secs = ratio * duration;
                                    let _ = player_cmd_tx.send(PlayerCommand::Seek(target_secs));
                                }
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if rect_contains(chunks.footer, cx, cy) {
                    let inner = get_inner_rect(chunks.footer);
                    let footer_parts = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(8),
                            Constraint::Length(1),
                            Constraint::Min(1),
                        ])
                        .split(inner);
                    if rect_contains(footer_parts[2], cx, cy) {
                        let rows = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Min(0),
                            ])
                            .split(footer_parts[2]);
                        if cy == rows[2].y && cx >= rows[2].x + rows[2].width.saturating_sub(12) && cx < rows[2].x + rows[2].width {
                            self.adjust_volume(-1.0, player_cmd_tx);
                            return true;
                        }
                    }
                }

                if rect_contains(chunks.sidebar, cx, cy) {
                    let max = self.state.sidebar_items.len().saturating_sub(1);
                    if self.state.sidebar_index < max {
                        self.state.sidebar_index += 1;
                    }
                } else if rect_contains(chunks.tracklist, cx, cy) {
                    self.state.increment_active_track_index();
                } else if self.state.queue_visible && rect_contains(chunks.queue, cx, cy) {
                    if self.state.queue_visible {
                        let max = self.state.queue.len().saturating_sub(1);
                        if self.state.queue_index < max {
                            self.state.queue_index += 1;
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if rect_contains(chunks.footer, cx, cy) {
                    let inner = get_inner_rect(chunks.footer);
                    let footer_parts = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(8),
                            Constraint::Length(1),
                            Constraint::Min(1),
                        ])
                        .split(inner);
                    if rect_contains(footer_parts[2], cx, cy) {
                        let rows = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Length(1),
                                Constraint::Min(0),
                            ])
                            .split(footer_parts[2]);
                        if cy == rows[2].y && cx >= rows[2].x + rows[2].width.saturating_sub(12) && cx < rows[2].x + rows[2].width {
                            self.adjust_volume(1.0, player_cmd_tx);
                            return true;
                        }
                    }
                }

                if rect_contains(chunks.sidebar, cx, cy) {
                    if self.state.sidebar_index > 0 {
                        self.state.sidebar_index -= 1;
                    }
                } else if rect_contains(chunks.tracklist, cx, cy) {
                    self.state.decrement_active_track_index();
                } else if self.state.queue_visible && rect_contains(chunks.queue, cx, cy) {
                    if self.state.queue_visible {
                        if self.state.queue_index > 0 {
                            self.state.queue_index -= 1;
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }
}

fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

fn get_inner_rect(area: Rect) -> Rect {
    if area.width >= 2 && area.height >= 2 {
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: area.height - 2,
        }
    } else {
        area
    }
}

fn tracklist_click_index(box_area: Rect, cy: u16, offset: usize) -> Option<usize> {
    let inner = get_inner_rect(box_area);
    if cy < inner.y { return None; }
    let click_y = (cy - inner.y) as i32 - 2;
    if click_y < 0 { return None; }
    Some(offset + click_y as usize)
}
