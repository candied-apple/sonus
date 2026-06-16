use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::config;
use crate::state::app_state::AppState;
use crate::state::command_palette::{ConfirmAction, PaletteMode};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    // Determine the prompt message
    let prompt = match state.palette_mode {
        PaletteMode::CommandSelection => "> ".to_string(),
        PaletteMode::CreatePlaylistInput => "Enter playlist name: ".to_string(),
        PaletteMode::DeletePlaylistSelection => "Select playlist to delete: ".to_string(),
        PaletteMode::AddToPlaylistSelection => "Select playlist to add track to: ".to_string(),
        PaletteMode::SeekInput => "Enter seek time (seconds or mm:ss): ".to_string(),
        PaletteMode::SpotifyImportInput => "Enter Spotify playlist URL or ID: ".to_string(),
        PaletteMode::ThemeSelection => "Select theme: ".to_string(),
        PaletteMode::Confirmation(action) => match action {
            ConfirmAction::ClearQueue => "Clear playback queue? (y/n) ".to_string(),
            ConfirmAction::ClearHistory => "Clear recently played history? (y/n) ".to_string(),
            ConfirmAction::ClearCache => "Clear local audio cache? (y/n) ".to_string(),
            ConfirmAction::DeletePlaylist { id } => {
                let name = state.cached_playlists.as_ref()
                    .and_then(|list| list.iter().find(|(pid, _)| *pid == id).map(|(_, n)| n.as_str()))
                    .unwrap_or("playlist");
                format!("Delete playlist '{}'? (y/n) ", name)
            }
        },
        PaletteMode::ContextActions => "Select track/playlist action: ".to_string(),
    };

    let popup_area = if state.palette_items.is_empty() {
        centered_rect_fixed_height(55, 3, area)
    } else {
        centered_rect(55, 35, area)
    };
    f.render_widget(ratatui::widgets::Clear, popup_area);

    let border_style = Style::default().fg(config::color_accent());
    let block = Block::default()
        .title(" Command Palette ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let (input_area, show_list) = if state.palette_items.is_empty() {
        (inner, false)
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        // Render separator
        let separator = Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(config::color_inactive()));
        f.render_widget(separator, chunks[1]);

        (chunks[0], true)
    };

    // Render input text
    let input_line = Line::from(vec![
        Span::styled(&prompt, Style::default().fg(config::color_selected())),
        Span::styled(&state.palette_input, Style::default().fg(config::color_text())),
        Span::styled("█", Style::default().fg(config::color_accent())),
    ]);
    f.render_widget(Paragraph::new(input_line), input_area);

    if show_list {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        // Render option list
        let visible_rows = chunks[2].height as usize;
        let selected = state.palette_selected;
        
        // Calculate scroll offset if items exceed visible rows
        let offset = if selected >= visible_rows {
            selected - visible_rows + 1
        } else {
            0
        };

        let list_lines: Vec<Line> = state.palette_items
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_rows)
            .map(|(i, item)| {
                let is_selected = i == selected;
                let style = if is_selected {
                    Style::default()
                        .fg(config::color_selected())
                        .bg(config::color_inactive())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Line::from(Span::styled(format!("  {}", item), style))
            })
            .collect();

        f.render_widget(Paragraph::new(list_lines), chunks[2]);
    }
}

pub fn centered_rect(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(pct_y),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(pct_x),
            Constraint::Fill(1),
        ])
        .split(popup_layout[1])[1]
}

pub fn centered_rect_fixed_height(pct_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(pct_x),
            Constraint::Fill(1),
        ])
        .split(popup_layout[1])[1]
}
