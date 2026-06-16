use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders},
    Frame,
};

use crate::config;

pub fn format_duration(seconds: f64) -> String {
    let mins = (seconds as u64) / 60;
    let secs = (seconds as u64) % 60;
    format!("{:02}:{:02}", mins, secs)
}

pub fn render_bordered_block<'a>(area: Rect, f: &mut Frame, title: &str, focused: bool) -> Rect {
    let border_color = if focused { config::color_accent() } else { config::color_inactive() };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

pub fn ensure_scroll(selected: usize, offset: &mut usize, visible: usize) {
    if visible == 0 {
        return;
    }
    if selected >= offset.saturating_add(visible) {
        *offset = selected.saturating_add(1).saturating_sub(visible);
    }
    if selected < *offset {
        *offset = selected;
    }
}
