use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Modifier},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::config;
use crate::state::app_state::{AppState, Focus, SearchTab, TrackCategory, TrackItem};
use crate::ui::components::{ensure_scroll, render_bordered_block};

use crate::util::fit_to_width;

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    if area.width < 20 || area.height < 3 {
        return;
    }

    if state.active_page == crate::state::app_state::ActivePage::Search {
        render_search_page(f, area, state);
    } else if state.active_page == crate::state::app_state::ActivePage::Explore {
        render_explore_page(f, area, state);
    } else {
        if state.is_search_results() {
            render_dual_boxes(f, area, state);
        } else {
            render_single_box(f, area, state);
        }
    }
}

fn render_explore_page(f: &mut Frame, area: Rect, state: &mut AppState) {
    use crate::state::app_state::ExploreSection;
    use ratatui::widgets::Paragraph;
    use ratatui::text::{Line, Span};
    use ratatui::style::Style;

    let content_focused = state.focus == Focus::Tracklist;
    match state.explore_section {
        ExploreSection::ForYou => {
            let songs: Vec<&TrackItem> = state.explore_for_you.iter().filter(|t| t.category == TrackCategory::Song).map(|a| a.as_ref()).collect();
            let videos: Vec<&TrackItem> = state.explore_for_you.iter().filter(|t| t.category == TrackCategory::Video).map(|a| a.as_ref()).collect();
            let search_tab = state.search_tab;
            let song_index = state.song_index;
            let video_index = state.video_index;
            let song_offset = &mut state.song_offset;
            let video_offset = &mut state.video_offset;
            render_dual_track_boxes(
                f,
                area,
                "For You",
                &songs,
                &videos,
                search_tab,
                song_index,
                song_offset,
                video_index,
                video_offset,
                content_focused,
            );
        }
        ExploreSection::History => {
            if state.history.is_empty() {
                let content_inner = render_bordered_block(area, f, "History", content_focused);
                let msg = if config::use_nerd_font() { "  \u{f017}  Your history is empty" } else { "  Your history is empty" };
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(msg, Style::default().fg(config::color_inactive())))),
                    content_inner,
                );
            } else {
                let songs: Vec<&TrackItem> = state.history.iter().filter(|t| t.category == TrackCategory::Song).map(|a| a.as_ref()).collect();
                let videos: Vec<&TrackItem> = state.history.iter().filter(|t| t.category == TrackCategory::Video).map(|a| a.as_ref()).collect();
                let search_tab = state.search_tab;
                let song_index = state.song_index;
                let video_index = state.video_index;
                let song_offset = &mut state.song_offset;
                let video_offset = &mut state.video_offset;
                render_dual_track_boxes(
                    f,
                    area,
                    "History",
                    &songs,
                    &videos,
                    search_tab,
                    song_index,
                    song_offset,
                    video_index,
                    video_offset,
                    content_focused,
                );
            }
        }
        ExploreSection::TopArtists => {
            render_dual_artists_boxes(
                f,
                area,
                &state.explore_top_artists,
                &state.explore_top_channels,
                state,
                content_focused,
            );
        }
    }
}

fn render_search_page(f: &mut Frame, area: Rect, state: &mut AppState) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::Paragraph;
    use ratatui::text::{Line, Span};
    use ratatui::style::Style;
    use crate::state::app_state::Focus;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let is_search_focused = state.focus == Focus::Search;
    let search_title = " Search YouTube Music ";
    let search_inner = render_bordered_block(chunks[0], f, search_title, is_search_focused);

    let display = if state.search_query.is_empty() {
        if config::use_nerd_font() { "  \u{f002}  Search..." } else { "  Search..." }.to_string()
    } else {
        if config::use_nerd_font() {
            format!("  \u{f002}  {}", state.search_query)
        } else {
            format!("  {}", state.search_query)
        }
    };
    let cursor = if is_search_focused { "\u{2588}" } else { "" };
    let text = format!("{}{}", display, cursor);
    let search_style = if is_search_focused {
        Style::default().fg(config::color_accent())
    } else {
        Style::default().fg(config::color_border())
    };
    f.render_widget(Paragraph::new(Line::from(Span::styled(text, search_style))), search_inner);

    if state.is_search_results() {
        render_dual_boxes(f, chunks[1], state);
    } else if !state.tracks.is_empty() {
        render_single_box(f, chunks[1], state);
    } else if let Some(ref status) = state.status_message {
        let inner = render_bordered_block(chunks[1], f, " Search ", false);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(status.as_str(), Style::default().fg(config::color_inactive())))),
            inner,
        );
    } else if !state.search_query.is_empty() {
        let inner = render_bordered_block(chunks[1], f, " Search ", false);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" No results found", Style::default().fg(config::color_inactive())))),
            inner,
        );
    }
}

fn render_single_box(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focus == Focus::Tracklist;
    let title = if state.resize_mode {
        format!(" {} [RESIZING - Arrows to adjust, Esc/R to exit] ", state.view_title)
    } else {
        format!(" {} ", state.view_title)
    };
    let inner = render_bordered_block(area, f, &title, focused);

    if state.tracks.is_empty() {
        let msg = if let Some(ref status) = state.status_message {
            status.as_str()
        } else if state.search_query.is_empty() {
            " Search for music or select a playlist"
        } else {
            " No results found"
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(msg, Style::default().fg(config::color_inactive())))),
            inner,
        );
        return;
    }

    let track_refs: Vec<&TrackItem> = state.tracks.iter().map(|a| a.as_ref()).collect();
    let lines = render_track_rows(inner, &track_refs, state.track_index, &mut state.track_offset, focused);
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn render_dual_boxes(f: &mut Frame, area: Rect, state: &mut AppState) {
    let songs: Vec<&TrackItem> = state.tracks.iter().filter(|t| t.category == TrackCategory::Song).map(|a| a.as_ref()).collect();
    let videos: Vec<&TrackItem> = state.tracks.iter().filter(|t| t.category == TrackCategory::Video).map(|a| a.as_ref()).collect();
    let search_tab = state.search_tab;
    let song_index = state.song_index;
    let video_index = state.video_index;
    let song_offset = &mut state.song_offset;
    let video_offset = &mut state.video_offset;
    render_dual_track_boxes(
        f,
        area,
        &state.view_title,
        &songs,
        &videos,
        search_tab,
        song_index,
        song_offset,
        video_index,
        video_offset,
        state.focus == Focus::Tracklist,
    );
}

fn render_dual_track_boxes(
    f: &mut Frame,
    area: Rect,
    title: &str,
    songs: &[&TrackItem],
    videos: &[&TrackItem],
    search_tab: SearchTab,
    song_index: usize,
    song_offset: &mut usize,
    video_index: usize,
    video_offset: &mut usize,
    focused: bool,
) {
    let halves = Layout::vertical([
        Constraint::Length(area.height / 2),
        Constraint::Length(area.height - area.height / 2),
    ]).split(area);

    // Songs box
    let songs_focused = focused && search_tab == SearchTab::Songs;
    let songs_title = format!(" {}: Songs ", title);
    let songs_inner = render_bordered_block(halves[0], f, &songs_title, songs_focused);
    if songs.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" No results found", Style::default().fg(config::color_inactive())))),
            songs_inner,
        );
    } else {
        let lines = render_track_rows(songs_inner, songs, song_index, song_offset, songs_focused);
        f.render_widget(Paragraph::new(Text::from(lines)), songs_inner);
    }

    // Videos box
    let videos_focused = focused && search_tab == SearchTab::Videos;
    let videos_title = format!(" {}: Videos ", title);
    let videos_inner = render_bordered_block(halves[1], f, &videos_title, videos_focused);
    if videos.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" No results found", Style::default().fg(config::color_inactive())))),
            videos_inner,
        );
    } else {
        let lines = render_video_rows(videos_inner, videos, video_index, video_offset, videos_focused);
        f.render_widget(Paragraph::new(Text::from(lines)), videos_inner);
    }
}

fn render_dual_artists_boxes(
    f: &mut Frame,
    area: Rect,
    artists: &[(String, usize)],
    channels: &[(String, usize)],
    state: &AppState,
    focused: bool,
) {
    let halves = Layout::vertical([
        Constraint::Length(area.height / 2),
        Constraint::Length(area.height - area.height / 2),
    ]).split(area);

    // Artists box
    let artists_focused = focused && state.search_tab == SearchTab::Songs;
    let artists_inner = render_bordered_block(halves[0], f, " Top Artists ", artists_focused);
    if artists.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("  Play more tracks to see your top artists here", Style::default().fg(config::color_inactive())))),
            artists_inner,
        );
    } else {
        let lines = render_artist_rows(artists_inner, artists, state.explore_artist_index, artists_focused);
        f.render_widget(Paragraph::new(Text::from(lines)), artists_inner);
    }

    // Channels box
    let channels_focused = focused && state.search_tab == SearchTab::Videos;
    let channels_inner = render_bordered_block(halves[1], f, " Top Channels ", channels_focused);
    if channels.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("  Play more videos to see your top channels here", Style::default().fg(config::color_inactive())))),
            channels_inner,
        );
    } else {
        let lines = render_artist_rows(channels_inner, channels, state.explore_channel_index, channels_focused);
        f.render_widget(Paragraph::new(Text::from(lines)), channels_inner);
    }
}

fn render_artist_rows(
    area: Rect,
    items: &[(String, usize)],
    selected: usize,
    focused: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let height = area.height as usize;
    let start = selected.saturating_sub(height / 2);
    let end = (start + height).min(items.len());
    let start = end.saturating_sub(height).min(start);

    for idx in start..end {
        let (name, play_count) = &items[idx];
        let is_selected = idx == selected && focused;
        let prefix = if is_selected { if config::use_nerd_font() { "\u{f0da} " } else { "▶ " } } else { "  " };
        let style = if is_selected {
            Style::default().fg(config::color_accent()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(config::color_text())
        };
        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(config::color_accent())),
            Span::styled(format!("{}. ", idx + 1), Style::default().fg(config::color_inactive())),
            Span::styled(name.clone(), style),
            Span::styled(format!("  ({} plays)", play_count), Style::default().fg(config::color_inactive())),
        ]);
        lines.push(line);
    }
    lines
}

fn render_track_rows(
    area: Rect,
    tracks: &[&TrackItem],
    selected: usize,
    offset: &mut usize,
    focused: bool,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let inner_w = area.width as usize;
    let play_w = 1usize;
    let index_w = 2usize;
    let time_w = 8usize;
    let gap = 2usize;
    let remaining = inner_w.saturating_sub(play_w + 1 + index_w + time_w + gap * 4);
    let third = remaining / 3;
    let title_w = third.max(5);
    let artist_w = third.max(5);
    let album_w = remaining.saturating_sub(title_w + artist_w).max(5);

    let header = Line::from(vec![
        Span::raw(" "),
        Span::raw(" "),
        Span::styled(format!("{:>2}", "#"), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(fit_to_width("Title", title_w, ""), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(fit_to_width("Artist", artist_w, ""), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(fit_to_width("Album", album_w, ""), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(format!("{:>8}", "Time"), Style::default().fg(config::color_inactive())),
    ]);
    lines.push(header);

    let sep = Line::from(Span::styled(
        "─".repeat(inner_w),
        Style::default().fg(config::color_inactive()),
    ));
    lines.push(sep);

    let mut temp_offset = *offset;
    let visible = area.height.saturating_sub(4) as usize;

    if visible == 0 {
        return lines;
    }

    ensure_scroll(selected, &mut temp_offset, visible);
    *offset = temp_offset;

    for (local_idx, track) in tracks.iter().enumerate().skip(temp_offset).take(visible) {
        let is_selected = local_idx == selected;
        let style = if is_selected {
            if focused {
                Style::default()
                    .fg(config::color_selected())
                    .bg(config::color_inactive())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(config::color_border())
            }
        } else if track.is_playing {
            Style::default().fg(config::color_playing())
        } else {
            Style::default()
        };

        let play_indicator = if track.is_playing {
            Span::styled(if config::use_nerd_font() { "\u{f04b}" } else { "▶" }, Style::default().fg(config::color_playing()))
        } else {
            Span::raw(" ")
        };

        let title_width = UnicodeWidthStr::width(track.title.as_str());
        let title = if title_width > title_w {
            if is_selected {
                let char_count = track.title.chars().count();
                let max_offset = char_count.saturating_sub(3);
                let pace = 200u64;
                let elapsed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let scroll_offset = if max_offset > 0 {
                    ((elapsed / pace) as usize) % (max_offset + 1)
                } else {
                    0
                };
                let slice: String = track.title.chars().skip(scroll_offset).collect();
                fit_to_width(&slice, title_w, "")
            } else {
                fit_to_width(&track.title, title_w, "…")
            }
        } else {
            fit_to_width(&track.title, title_w, "")
        };

        let artist = fit_to_width(&track.artist, artist_w, "…");
        let fallback = if track.category == TrackCategory::Video { "Video" } else { "-" };
        let album = fit_to_width(track.album.as_deref().unwrap_or(fallback), album_w, "…");

        let spans = vec![
            play_indicator,
            Span::raw(" "),
            Span::styled(format!("{:>2}", local_idx + 1), Style::default().fg(config::color_inactive())),
            Span::raw("  "),
            Span::styled(title, style),
            Span::raw("  "),
            Span::styled(artist, Style::default().fg(config::color_border())),
            Span::raw("  "),
            Span::styled(album, Style::default().fg(config::color_border())),
            Span::raw("  "),
            Span::styled(format!("{:>8}", track.duration), Style::default().fg(config::color_inactive())),
        ];

        lines.push(Line::from(spans));
    }

    lines
}

fn render_video_rows(
    area: Rect,
    tracks: &[&TrackItem],
    selected: usize,
    offset: &mut usize,
    focused: bool,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let inner_w = area.width as usize;
    let play_w = 1usize;
    let index_w = 2usize;
    let time_w = 8usize;
    let gap = 2usize;
    let remaining = inner_w.saturating_sub(play_w + 1 + index_w + time_w + gap * 3);
    let title_w = remaining / 2;
    let channel_w = remaining.saturating_sub(title_w).max(5);

    let header = Line::from(vec![
        Span::raw(" "),
        Span::raw(" "),
        Span::styled(format!("{:>2}", "#"), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(fit_to_width("Title", title_w, ""), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(fit_to_width("Channel", channel_w, ""), Style::default().fg(config::color_inactive())),
        Span::raw("  "),
        Span::styled(format!("{:>8}", "Time"), Style::default().fg(config::color_inactive())),
    ]);
    lines.push(header);

    let sep = Line::from(Span::styled(
        "─".repeat(inner_w),
        Style::default().fg(config::color_inactive()),
    ));
    lines.push(sep);

    let mut temp_offset = *offset;
    let visible = area.height.saturating_sub(4) as usize;

    if visible == 0 {
        return lines;
    }

    ensure_scroll(selected, &mut temp_offset, visible);
    *offset = temp_offset;

    for (local_idx, track) in tracks.iter().enumerate().skip(temp_offset).take(visible) {
        let is_selected = local_idx == selected;
        let style = if is_selected {
            if focused {
                Style::default()
                    .fg(config::color_selected())
                    .bg(config::color_inactive())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(config::color_border())
            }
        } else if track.is_playing {
            Style::default().fg(config::color_playing())
        } else {
            Style::default()
        };

        let play_indicator = if track.is_playing {
            Span::styled(if config::use_nerd_font() { "\u{f04b}" } else { "▶" }, Style::default().fg(config::color_playing()))
        } else {
            Span::raw(" ")
        };

        let title_width = UnicodeWidthStr::width(track.title.as_str());
        let title = if title_width > title_w {
            if is_selected {
                let char_count = track.title.chars().count();
                let max_offset = char_count.saturating_sub(3);
                let pace = 200u64;
                let elapsed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let scroll_offset = if max_offset > 0 {
                    ((elapsed / pace) as usize) % (max_offset + 1)
                } else {
                    0
                };
                let slice: String = track.title.chars().skip(scroll_offset).collect();
                fit_to_width(&slice, title_w, "")
            } else {
                fit_to_width(&track.title, title_w, "…")
            }
        } else {
            fit_to_width(&track.title, title_w, "")
        };

        let channel = fit_to_width(&track.artist, channel_w, "…");

        let spans = vec![
            play_indicator,
            Span::raw(" "),
            Span::styled(format!("{:>2}", local_idx + 1), Style::default().fg(config::color_inactive())),
            Span::raw("  "),
            Span::styled(title, style),
            Span::raw("  "),
            Span::styled(channel, Style::default().fg(config::color_border())),
            Span::raw("  "),
            Span::styled(format!("{:>8}", track.duration), Style::default().fg(config::color_inactive())),
        ];

        lines.push(Line::from(spans));
    }

    lines
}
