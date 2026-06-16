use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::{LineGauge, Paragraph},
    Frame,
};

use crate::config;
use crate::state::app_state::{AppState, PlayStatus, RepeatMode};
use crate::ui::components::{format_duration, render_bordered_block};

pub fn render(f: &mut Frame, area: Rect, state: &AppState, cover_image: &mut Option<ratatui_image::protocol::StatefulProtocol>) {
    if area.height < 4 {
        return;
    }
    let inner = render_bordered_block(area, f, " Now Playing ", false);

    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1), // Left padding
            Constraint::Length(6), // Image
            Constraint::Length(1), // Spacer
            Constraint::Min(1),    // Text and controls
        ])
        .split(inner);

    if let Some(protocol) = cover_image {
        let image_widget = ratatui_image::StatefulImage::new();
        f.render_stateful_widget(image_widget, parts[1], protocol);
    } else {
        let placeholder = Paragraph::new("\n ♫ ")
            .style(Style::default().fg(config::color_inactive()))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(placeholder, parts[1]);
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(parts[3]);

    render_track_line(f, rows[0], state);
    render_progress_row(f, rows[1], state);
    render_controls_row(f, rows[2], state);
}

fn render_track_line(f: &mut Frame, area: Rect, state: &AppState) {
    let text = match &state.player.current_track {
        Some((title, artist)) => Line::from(vec![
            Span::styled(
                format!(" {}  –  {}", title, artist),
                Style::default().fg(config::color_text()).add_modifier(Modifier::BOLD),
            ),
        ]),
        None => Line::from(Span::styled(
            " No track playing",
            Style::default().fg(config::color_inactive()),
        )),
    };
    f.render_widget(Paragraph::new(text), area);
}

fn render_progress_row(f: &mut Frame, area: Rect, state: &AppState) {
    if state.player.duration <= 0.0 {
        return;
    }

    let ratio = (state.player.position / state.player.duration).clamp(0.0, 1.0);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let gauge = LineGauge::default()
        .filled_style(Style::default().fg(config::color_accent()))
        .unfilled_style(Style::default().fg(config::color_inactive()))
        .label("")
        .ratio(ratio);
    f.render_widget(gauge, cols[0]);
}

fn render_controls_row(f: &mut Frame, area: Rect, state: &AppState) {
    let use_nerd = config::use_nerd_font();
    let is_playing = state.player.status == PlayStatus::Playing;

    let shuffle_state = if state.player.shuffle { "on" } else { "off" };
    let repeat_state = match state.player.repeat {
        RepeatMode::None => "off",
        RepeatMode::One => "one",
        RepeatMode::All => "all",
    };
    let auto_state = if state.auto_play { "on" } else { "off" };

    let spans = if use_nerd {
        let play_icon = if is_playing { "\u{f04c}" } else { "\u{f04b}" };
        let prev_icon = "\u{f048}";
        let next_icon = "\u{f051}";
        let shuffle_icon = "\u{f074}";
        let repeat_icon = "\u{f01e}";
        let radio_icon = "\u{f0e7}";

        vec![
            Span::raw(" "),
            Span::styled(prev_icon, Style::default().fg(config::color_border())),
            Span::raw("  "),
            Span::styled(play_icon, Style::default().fg(config::color_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(next_icon, Style::default().fg(config::color_border())),
            Span::raw("   "),
            Span::styled(format!("{} {}", shuffle_icon, shuffle_state), Style::default().fg(if state.player.shuffle { config::color_accent() } else { config::color_inactive() })),
            Span::raw("   "),
            Span::styled(format!("{} {}", repeat_icon, repeat_state), Style::default().fg(if state.player.repeat != RepeatMode::None { config::color_accent() } else { config::color_inactive() })),
            Span::raw("   "),
            Span::styled(format!("{} {}", radio_icon, auto_state), Style::default().fg(if state.auto_play { config::color_accent() } else { config::color_inactive() })),
        ]
    } else {
        let play_icon = if is_playing { "||" } else { ">" };
        let prev_icon = "|<";
        let next_icon = ">|";
        let shuffle_icon = "Shuffle:";
        let repeat_icon = "Repeat:";
        let radio_icon = "Auto:";

        vec![
            Span::raw(" "),
            Span::styled(prev_icon, Style::default().fg(config::color_border())),
            Span::raw("  "),
            Span::styled(play_icon, Style::default().fg(config::color_accent()).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(next_icon, Style::default().fg(config::color_border())),
            Span::raw("   "),
            Span::styled(format!("{} {}", shuffle_icon, shuffle_state), Style::default().fg(if state.player.shuffle { config::color_accent() } else { config::color_inactive() })),
            Span::raw("   "),
            Span::styled(format!("{} {}", repeat_icon, repeat_state), Style::default().fg(if state.player.repeat != RepeatMode::None { config::color_accent() } else { config::color_inactive() })),
            Span::raw("   "),
            Span::styled(format!("{} {}", radio_icon, auto_state), Style::default().fg(if state.auto_play { config::color_accent() } else { config::color_inactive() })),
        ]
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(32),
            Constraint::Length(2),
        ])
        .split(area);

    f.render_widget(Paragraph::new(Line::from(spans)), cols[0]);

    let vol = (state.player.volume * 100.0) as u8;
    let time_text = if state.player.duration > 0.0 {
        let pos_label = format_duration(state.player.position);
        let dur_label = format_duration(state.player.duration);
        format!("{} / {}    ", pos_label, dur_label)
    } else {
        "".to_string()
    };
    let vol_text = if use_nerd {
        format!("{}\u{f028} {}%", time_text, vol)
    } else {
        format!("{}Vol {}%", time_text, vol)
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            vol_text,
            Style::default().fg(config::color_accent()),
        )))
        .alignment(ratatui::layout::Alignment::Right),
        cols[1],
    );
}
