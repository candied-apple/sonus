use ratatui::{
    layout::Rect,
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config;
use crate::state::app_state::{AppState, Focus};
use crate::ui::components::{ensure_scroll, render_bordered_block};

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    if area.width < 10 || area.height < 3 {
        return;
    }

    let focused = state.focus == Focus::Sidebar;
    let inner = render_bordered_block(area, f, " Navigation ", focused);

    if state.sidebar_items.is_empty() {
        return;
    }

    let items = &state.sidebar_items;
    let mut lines = Vec::new();
    let selected = state.sidebar_index;
    let mut offset = state.sidebar_offset;
    let visible = inner.height as usize;

    let effective_selected = if selected >= 4 { selected + 1 } else { selected };
    ensure_scroll(effective_selected, &mut offset, visible);
    state.sidebar_offset = offset;

    for (i, item) in items.iter().enumerate() {
        if i == 4 {
            let divider_style = Style::default().fg(config::color_border());
            let divider_line = "─".repeat(inner.width as usize);
            lines.push(Line::from(Span::styled(divider_line, divider_style)));
        }

        let is_selected = i == selected;
        let prefix = if config::use_nerd_font() {
            match item.playlist_id.as_deref() {
                Some("search") => "  \u{f002} ",
                Some("foryou") => "  \u{f0d0} ",
                Some("history") => "  \u{f017} ",
                Some("topartists") => "  \u{f007} ",
                _ => "  \u{f07b} ",
            }
        } else {
            match item.playlist_id.as_deref() {
                Some(_) => "  ",
                None => "  - ",
            }
        };

        let style = if is_selected {
            if focused {
                Style::default()
                    .fg(config::color_selected())
                    .bg(config::color_inactive())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(config::color_accent())
                    .add_modifier(Modifier::BOLD)
            }
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(format!("{}{}", prefix, item.label), style)));
    }

    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(offset)
        .take(visible)
        .collect();

    f.render_widget(Paragraph::new(visible_lines), inner);
}
