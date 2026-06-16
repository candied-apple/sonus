use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config;
use crate::state::app_state::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    if area.width < 20 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(60)])
        .split(area);

    render_title(f, chunks[0], state);
    render_toggles(f, chunks[1], state);
}

fn render_title(f: &mut Frame, area: Rect, state: &AppState) {
    use ratatui::style::Modifier;

    let title_style = Style::default()
        .fg(config::color_accent())
        .add_modifier(Modifier::BOLD);

    let title_text = if config::use_nerd_font() { "  \u{f025}  Sonus" } else { "  ♫  Sonus" }.to_string();

    let mut spans = vec![
        Span::styled(title_text, title_style),
    ];

    if let Some(ref latest) = state.new_version_available {
        let update_style = Style::default()
            .fg(config::color_selected())
            .add_modifier(Modifier::ITALIC);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(format!("(Update available: {})", latest), update_style));
    }

    let line = Line::from(spans);

    f.render_widget(Paragraph::new(line), area);
}

fn render_toggles(f: &mut Frame, area: Rect, state: &AppState) {
    let q_icon = if config::use_nerd_font() { "\u{f0ca}" } else { "☰" };
    let l_icon = if config::use_nerd_font() { "\u{f025}" } else { "♫" };

    let q_style = if state.queue_visible {
        Style::default().fg(config::color_accent())
    } else {
        Style::default().fg(config::color_inactive())
    };
    let l_style = if state.lyrics_visible {
        Style::default().fg(config::color_accent())
    } else {
        Style::default().fg(config::color_inactive())
    };
    let h_style = if state.help_visible {
        Style::default().fg(config::color_accent())
    } else {
        Style::default().fg(config::color_inactive())
    };

    let queue_text = if config::use_nerd_font() { format!(" {}  Queue", q_icon) } else { " Queue ".to_string() };
    let lyrics_text = if config::use_nerd_font() { format!(" {}  Lyrics", l_icon) } else { " Lyrics ".to_string() };
    let help_text = "[?]".to_string();

    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled(queue_text, q_style),
        Span::raw("  "),
        Span::styled(lyrics_text, l_style),
        Span::raw("  "),
        Span::styled(help_text, h_style),
    ]);

    f.render_widget(Paragraph::new(line).alignment(Alignment::Right), area);
}
