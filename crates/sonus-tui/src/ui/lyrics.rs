use ratatui::{
    layout::Rect,
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config;
use crate::state::app_state::AppState;
use crate::ui::components::render_bordered_block;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    if area.width < 10 || area.height < 3 {
        return;
    }

    let inner = render_bordered_block(area, f, " Lyrics ", true);

    if let Some(synced) = &state.synced_lyrics {
        if !synced.is_empty() {
            let active_idx = synced
                .partition_point(|line| line.timestamp <= state.player.position)
                .saturating_sub(1);

            let mid = (inner.height / 2) as usize;

            let mut lines: Vec<Line> = Vec::with_capacity(synced.len() + mid);

            // Synced lyrics (no top padding)
            for (idx, line) in synced.iter().enumerate() {
                if idx == active_idx {
                    lines.push(Line::from(Span::styled(
                        format!(" {} ", line.text),
                        Style::default().fg(config::color_accent()).add_modifier(Modifier::BOLD),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!(" {} ", line.text),
                        Style::default().fg(config::color_inactive()),
                    )));
                }
            }

            // Pad bottom with empty lines to allow scrolling the last line to the center
            for _ in 0..mid {
                lines.push(Line::from(""));
            }

            let scroll_y = active_idx.saturating_sub(mid) as u16;

            f.render_widget(
                Paragraph::new(lines)
                    .scroll((scroll_y, 0))
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                inner,
            );
            return;
        }
    }

    match &state.lyrics_text {
        Some(lyrics) if !lyrics.is_empty() => {
            let lines: Vec<Line> = lyrics
                .lines()
                .map(|l| Line::from(Span::raw(l.to_string())))
                .collect();
            f.render_widget(Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: true }), inner);
        }
        _ => {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " No lyrics available",
                    Style::default().fg(config::color_inactive()),
                ))).wrap(ratatui::widgets::Wrap { trim: true }),
                inner,
            );
        }
    }
}
