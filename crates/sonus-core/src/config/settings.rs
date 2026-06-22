use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use super::color::{Color, parse_color};

#[derive(Debug, Clone)]
pub struct Config {
    pub use_nerd_font: bool,
    pub color_accent: Color,
    pub color_selected: Color,
    pub color_playing: Color,
    pub color_inactive: Color,
    pub color_border: Color,
    pub color_error: Color,
    pub color_success: Color,
    pub color_text: Color,
    pub cache_limit_bytes: u64,
    pub default_volume: f64,
    pub history_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            use_nerd_font: true,
            color_accent: Color::Cyan,
            color_selected: Color::LightYellow,
            color_playing: Color::Green,
            color_inactive: Color::DarkGray,
            color_border: Color::Gray,
            color_error: Color::Red,
            color_success: Color::Green,
            color_text: Color::White,
            cache_limit_bytes: 1000 * 1024 * 1024, // 1000 MB
            default_volume: 0.8,
            history_limit: 100,
        }
    }
}

static CURRENT_VOLUME: AtomicU64 = AtomicU64::new(f64::NAN.to_bits());

pub fn get_config() -> &'static RwLock<Config> {
    static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let config = load_config().unwrap_or_default();
        CURRENT_VOLUME.store(config.default_volume.to_bits(), Ordering::Relaxed);
        RwLock::new(config)
    })
}

fn load_config() -> Option<Config> {
    let config_dir = dirs::config_dir()?.join("sonus");
    let _ = std::fs::create_dir_all(&config_dir);
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        let default_toml = r##"# Sonus configuration file

use_nerd_font = true

# The default volume level when player starts (between 0.0 and 1.0)
default_volume = 0.8

# The audio cache limit in megabytes (MB)
cache_limit_mb = 1000

# The maximum number of history songs to keep
history_limit = 100

# Colors can be hex (e.g. "#00FFFF") or standard names (e.g. "cyan", "red", "green", "white")
color_accent = "cyan"
color_selected = "lightyellow"
color_playing = "green"
color_inactive = "darkgray"
color_border = "gray"
color_error = "red"
color_success = "green"
color_text = "white"
"##;
        let _ = std::fs::write(&config_path, default_toml);
        return Some(Config::default());
    }

    let content = std::fs::read_to_string(config_path).ok()?;
    let mut config = Config::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'').trim();
            match key {
                "use_nerd_font" => {
                    if let Ok(b) = val.parse::<bool>() {
                        config.use_nerd_font = b;
                    }
                }
                "default_volume" => {
                    if let Ok(v) = val.parse::<f64>() {
                        config.default_volume = v.clamp(0.0, 1.0);
                    }
                }
                "cache_limit_mb" => {
                    if let Ok(limit_mb) = val.parse::<u64>() {
                        config.cache_limit_bytes = limit_mb * 1024 * 1024;
                    }
                }
                "history_limit" => {
                    if let Ok(limit) = val.parse::<usize>() {
                        config.history_limit = limit;
                    }
                }
                "color_accent" => config.color_accent = parse_color(val),
                "color_selected" => config.color_selected = parse_color(val),
                "color_playing" => config.color_playing = parse_color(val),
                "color_inactive" => config.color_inactive = parse_color(val),
                "color_border" => config.color_border = parse_color(val),
                "color_error" => config.color_error = parse_color(val),
                "color_success" => config.color_success = parse_color(val),
                "color_text" => config.color_text = parse_color(val),
                _ => {}
            }
        }
    }

    // Respect environmental override as well
    if std::env::var("SONUS_NO_NERD").is_ok() {
        config.use_nerd_font = false;
    }

    Some(config)
}

macro_rules! read_config {
    () => {
        get_config().read().unwrap_or_else(|e| e.into_inner())
    };
}

pub fn color_accent() -> Color { read_config!().color_accent }
pub fn color_selected() -> Color { read_config!().color_selected }
pub fn color_playing() -> Color { read_config!().color_playing }
pub fn color_inactive() -> Color { read_config!().color_inactive }
pub fn color_border() -> Color { read_config!().color_border }
pub fn color_success() -> Color { read_config!().color_success }
pub fn color_text() -> Color { read_config!().color_text }

pub fn use_nerd_font() -> bool {
    read_config!().use_nerd_font
}

pub fn cache_limit_bytes() -> u64 {
    read_config!().cache_limit_bytes
}

pub fn history_limit() -> usize {
    read_config!().history_limit
}

pub fn default_volume() -> f64 {
    let bits = CURRENT_VOLUME.load(Ordering::Relaxed);
    if bits == f64::NAN.to_bits() {
        read_config!().default_volume
    } else {
        f64::from_bits(bits)
    }
}

pub fn write_config_atomically(config_path: &std::path::Path, content: &str) {
    let tmp_path = config_path.with_extension("toml.tmp");
    if let Err(e) = std::fs::write(&tmp_path, content) {
        let _ = std::fs::remove_file(&tmp_path);
        crate::log!("Failed to write config: {}", e);
    } else if let Err(e) = std::fs::rename(&tmp_path, config_path) {
        crate::log!("Failed to rename config: {}", e);
    }
}

pub fn update_config_line(config_path: &std::path::Path, key: &str, new_line: &str) {
    if let Ok(content) = std::fs::read_to_string(config_path) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut found = false;
        for line in &mut lines {
            if matches_config_key(line.trim(), key) {
                *line = new_line.to_string();
                found = true;
                break;
            }
        }
        if !found {
            lines.push(new_line.to_string());
        }
        write_config_atomically(config_path, &lines.join("\n"));
    }
}

pub fn matches_config_key(trimmed: &str, key: &str) -> bool {
    trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with('=')
}

pub fn update_default_volume(v: f64) {
    let v = v.clamp(0.0, 1.0);
    CURRENT_VOLUME.store(v.to_bits(), Ordering::Relaxed);

    if let Some(config_dir) = dirs::config_dir().map(|p| p.join("sonus")) {
        let config_path = config_dir.join("config.toml");
        update_config_line(&config_path, "default_volume", &format!("default_volume = {:.2}", v));
    }
}
