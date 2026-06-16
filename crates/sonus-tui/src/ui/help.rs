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

pub fn render(f: &mut Frame, area: Rect, _state: &AppState) {
    if area.width < 10 || area.height < 3 {
        return;
    }

    let inner = render_bordered_block(area, f, " ? Help ", true);

    let sections = vec![
        ("Navigation", vec![
            ("Tab / S-Tab", "Cycle active panels"),
            ("/", "Focus search bar"),
            ("Up / Down", "Move selection"),
            ("Enter", "Select / Play track"),
            ("Esc", "Close menu / Clear"),
        ]),
        ("Playback", vec![
            ("Space", "Play / Pause"),
            ("n / p", "Next / Prev track"),
            ("s", "Toggle shuffle"),
            ("r", "Cycle repeat mode"),
            ("+ / -", "Volume control"),
        ]),
        ("Panels", vec![
            ("q", "Toggle queue view"),
            ("h", "Toggle history view"),
            ("l", "Toggle lyrics view"),
            ("?", "Toggle help view"),
        ]),
        ("Resizing", vec![
            ("Ctrl+R", "Toggle resize mode"),
            ("Arrows", "Adjust widths (in resize mode)"),
        ]),
        ("Command Palette", vec![
            (": / Ctrl+P", "Open command palette"),
        ]),
        ("Context Menu", vec![
            ("c", "Open track context menu"),
            ("Up / Down", "Navigate menu"),
            ("Enter / Esc", "Execute / Close menu"),
        ]),
        ("General", vec![
            ("Ctrl+C", "Quit application"),
        ]),
    ];

    let mut shortcuts = Vec::new();
    for (sec_title, items) in sections {
        shortcuts.push(Line::from(Span::styled(
            format!(" {}", sec_title),
            Style::default().fg(config::color_accent()).add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in items {
            shortcuts.push(Line::from(vec![
                Span::styled(format!("  {}  ", key), Style::default().fg(config::color_text()).add_modifier(Modifier::BOLD)),
                Span::styled(desc.to_string(), Style::default().fg(config::color_inactive())),
            ]));
        }
        shortcuts.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(shortcuts).wrap(ratatui::widgets::Wrap { trim: false }), inner);
}
