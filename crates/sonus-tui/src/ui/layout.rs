use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::state::app_state::AppState;

pub struct LayoutChunks {
    pub header: Rect,
    pub sidebar: Rect,
    pub tracklist: Rect,
    pub queue: Rect,
    pub footer: Rect,
}

pub fn calculate_layout(area: Rect, state: &AppState) -> LayoutChunks {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(5),
        ])
        .split(area);

    let show_right_panel = state.queue_visible || state.lyrics_visible || state.help_visible;
    
    let constraints = if show_right_panel {
        vec![
            Constraint::Length(state.sidebar_width),
            Constraint::Min(1),
            Constraint::Length(state.right_panel_width),
        ]
    } else {
        vec![
            Constraint::Length(state.sidebar_width),
            Constraint::Min(1),
        ]
    };

    let body_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(vertical[1]);

    let sidebar = body_h[0];
    let tracklist = body_h[1];
    let queue = if show_right_panel { body_h[2] } else { Rect::default() };

    LayoutChunks {
        header: vertical[0],
        sidebar,
        tracklist,
        queue,
        footer: vertical[2],
    }
}
