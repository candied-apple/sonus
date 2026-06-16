pub mod command_palette;
pub mod components;
pub mod header;
pub mod help;
pub mod layout;
pub mod lyrics;
pub mod now_playing;
pub mod queue;
pub mod sidebar;
pub mod tracklist;

use ratatui::{layout::Rect, Frame};

use crate::state::app_state::AppState;

pub fn render(f: &mut Frame, state: &mut AppState, cover_image: &mut Option<ratatui_image::protocol::StatefulProtocol>) {
    crate::config::refresh_theme();
    state.terminal_area = f.area();
    let chunks = layout::calculate_layout(state.terminal_area, state);

    header::render(f, chunks.header, state);
    sidebar::render(f, chunks.sidebar, state);
    tracklist::render(f, chunks.tracklist, state);

    if chunks.queue.width > 0 {
        if state.queue_visible {
            queue::render(f, chunks.queue, state);
        } else if state.lyrics_visible {
            lyrics::render(f, chunks.queue, state);
        } else if state.help_visible {
            help::render(f, chunks.queue, state);
        }
    }



    if state.palette_visible {
        command_palette::render(f, f.area(), state);
    }

    if let Some(import) = &state.spotify_import {
        render_import_progress(f, f.area(), import);
    }

    now_playing::render(f, chunks.footer, state, cover_image);
}

fn render_import_progress(f: &mut Frame, area: Rect, import: &crate::state::app_state::SpotifyImportState) {
    use ratatui::{
        layout::{Constraint, Direction, Layout, Alignment},
        style::{Style, Modifier},
        widgets::{Block, Borders, Paragraph, Clear, Gauge},
    };
    use crate::config;

    let popup_area = command_palette::centered_rect(60, 25, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Spotify Playlist Import ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(config::color_success()));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    // Title / Playlist name
    let playlist_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("Importing: ", Style::default().fg(config::color_inactive())),
        ratatui::text::Span::styled(&import.playlist_name, Style::default().fg(config::color_selected()).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(playlist_line).alignment(Alignment::Center), chunks[0]);

    // Gauge progress
    let pct = if import.total_tracks > 0 {
        (import.completed as f64 / import.total_tracks as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let label = if import.total_tracks > 0 {
        format!("{} / {} tracks resolved", import.completed, import.total_tracks)
    } else {
        "Connecting...".to_string()
    };
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(config::color_success()).bg(config::color_inactive()))
        .ratio(pct)
        .label(label);
    f.render_widget(gauge, chunks[2]);

    // Current track being resolved
    let track_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("Resolving: ", Style::default().fg(config::color_inactive())),
        ratatui::text::Span::styled(&import.current_track_name, Style::default().fg(config::color_text())),
    ]);
    f.render_widget(Paragraph::new(track_line).alignment(Alignment::Center), chunks[3]);
}

