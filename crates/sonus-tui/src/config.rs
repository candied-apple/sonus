use ratatui::style::Color as RatatuiColor;
use sonus_core::config::Color as CoreColor;

use std::cell::Cell;

thread_local! {
    static CACHED_COLORS: Cell<Option<CachedColors>> = const { Cell::new(None) };
}

#[derive(Debug, Clone, Copy)]
struct CachedColors {
    accent: RatatuiColor,
    selected: RatatuiColor,
    playing: RatatuiColor,
    inactive: RatatuiColor,
    border: RatatuiColor,
    success: RatatuiColor,
    text: RatatuiColor,
}

pub fn refresh_theme() {
    CACHED_COLORS.with(|c| {
        c.set(Some(CachedColors {
            accent: to_ratatui_color(sonus_core::config::color_accent()),
            selected: to_ratatui_color(sonus_core::config::color_selected()),
            playing: to_ratatui_color(sonus_core::config::color_playing()),
            inactive: to_ratatui_color(sonus_core::config::color_inactive()),
            border: to_ratatui_color(sonus_core::config::color_border()),
            success: to_ratatui_color(sonus_core::config::color_success()),
            text: to_ratatui_color(sonus_core::config::color_text()),
        }));
    });
}

fn get_color<F>(f: F) -> RatatuiColor
where
    F: FnOnce(&CachedColors) -> RatatuiColor,
{
    CACHED_COLORS.with(|c| {
        if let Some(cached) = c.get() {
            f(&cached)
        } else {
            refresh_theme();
            c.get().map(|cached| f(&cached)).unwrap_or(RatatuiColor::Reset)
        }
    })
}

pub fn color_accent() -> RatatuiColor { get_color(|c| c.accent) }
pub fn color_selected() -> RatatuiColor { get_color(|c| c.selected) }
pub fn color_playing() -> RatatuiColor { get_color(|c| c.playing) }
pub fn color_inactive() -> RatatuiColor { get_color(|c| c.inactive) }
pub fn color_border() -> RatatuiColor { get_color(|c| c.border) }
pub fn color_success() -> RatatuiColor { get_color(|c| c.success) }
pub fn color_text() -> RatatuiColor { get_color(|c| c.text) }

pub fn use_nerd_font() -> bool { sonus_core::config::use_nerd_font() }
pub fn history_limit() -> usize { sonus_core::config::history_limit() }
pub fn update_default_volume(v: f64) { sonus_core::config::update_default_volume(v) }
pub fn update_theme(theme_name: &str) -> Result<(), String> {
    sonus_core::config::update_theme(theme_name)?;
    refresh_theme();
    Ok(())
}

pub const PREDEFINED_THEMES: &[sonus_core::config::Theme] = sonus_core::config::PREDEFINED_THEMES;

fn to_ratatui_color(c: CoreColor) -> RatatuiColor {
    match c {
        CoreColor::Reset => RatatuiColor::Reset,
        CoreColor::Black => RatatuiColor::Black,
        CoreColor::Red => RatatuiColor::Red,
        CoreColor::Green => RatatuiColor::Green,
        CoreColor::Yellow => RatatuiColor::Yellow,
        CoreColor::Blue => RatatuiColor::Blue,
        CoreColor::Magenta => RatatuiColor::Magenta,
        CoreColor::Cyan => RatatuiColor::Cyan,
        CoreColor::Gray => RatatuiColor::Gray,
        CoreColor::DarkGray => RatatuiColor::DarkGray,
        CoreColor::LightRed => RatatuiColor::LightRed,
        CoreColor::LightGreen => RatatuiColor::LightGreen,
        CoreColor::LightYellow => RatatuiColor::LightYellow,
        CoreColor::LightBlue => RatatuiColor::LightBlue,
        CoreColor::LightMagenta => RatatuiColor::LightMagenta,
        CoreColor::LightCyan => RatatuiColor::LightCyan,
        CoreColor::White => RatatuiColor::White,
        CoreColor::Rgb(r, g, b) => RatatuiColor::Rgb(r, g, b),
        CoreColor::Indexed(i) => RatatuiColor::Indexed(i),
    }
}
