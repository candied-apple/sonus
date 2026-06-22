pub mod color;
pub mod settings;
pub mod theme;

pub use color::{Color, color_to_string};
pub use settings::{
    Config, get_config, use_nerd_font, cache_limit_bytes, history_limit, default_volume,
    update_default_volume, color_accent, color_selected, color_playing, color_inactive,
    color_border, color_success, color_text,
};
pub use theme::{Theme, PREDEFINED_THEMES, update_theme};
