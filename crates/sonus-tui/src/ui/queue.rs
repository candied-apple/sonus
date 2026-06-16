use ratatui::{
    layout::Rect,
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config;
use crate::state::app_state::AppState;
use crate::ui::components::{ensure_scroll, render_bordered_block};
use crate::util::fit_to_width;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    if area.width < 10 || area.height < 3 {
        return;
    }

    let is_focused = state.focus == crate::state::app_state::Focus::Queue;
    let inner = render_bordered_block(area, f, " Queue ", is_focused);

    if state.queue.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Queue is empty",
                Style::default().fg(config::color_inactive()),
            ))),
            inner,
        );
        return;
    }

    let items = &state.queue;
    let selected = state.queue_index;
    let mut offset = 0usize;
    let visible = inner.height as usize;

    ensure_scroll(selected, &mut offset, visible);

    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, track)| {
            let is_playing = i == selected && state.player.current_track
                .as_ref()
                .map(|(t, _)| *t == track.title)
                .unwrap_or(false);
            let is_selected = i == selected;

            let prefix = if is_playing {
                if config::use_nerd_font() {
                    "\u{f04b} "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };
            let style = if is_selected {
                Style::default()
                    .fg(config::color_selected())
                    .bg(config::color_inactive())
                    .add_modifier(Modifier::BOLD)
            } else if is_playing {
                Style::default().fg(config::color_playing())
            } else {
                Style::default()
            };

            let title_w = (area.width as usize).saturating_sub(5);
            Line::from(vec![
                Span::raw(prefix),
                Span::styled(fit_to_width(&track.title, title_w, "…"), style),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}


