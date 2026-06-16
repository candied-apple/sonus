#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

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

fn parse_color(s: &str) -> Color {
    match s.trim().to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" | "dark_gray" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        other => {
            if other.starts_with('#') && other.len() == 7 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&other[1..3], 16),
                    u8::from_str_radix(&other[3..5], 16),
                    u8::from_str_radix(&other[5..7], 16),
                ) {
                    return Color::Rgb(r, g, b);
                }
            }
            Color::White
        }
    }
}

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

static CURRENT_VOLUME: AtomicU64 = AtomicU64::new(f64::NAN.to_bits());

fn get_config() -> &'static RwLock<Config> {
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

fn write_config_atomically(config_path: &std::path::Path, content: &str) {
    let tmp_path = config_path.with_extension("toml.tmp");
    if let Err(e) = std::fs::write(&tmp_path, content) {
        let _ = std::fs::remove_file(&tmp_path);
        crate::log!("Failed to write config: {}", e);
    } else if let Err(e) = std::fs::rename(&tmp_path, config_path) {
        crate::log!("Failed to rename config: {}", e);
    }
}

fn update_config_line(config_path: &std::path::Path, key: &str, new_line: &str) {
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

fn matches_config_key(trimmed: &str, key: &str) -> bool {
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

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub color_accent: Color,
    pub color_selected: Color,
    pub color_playing: Color,
    pub color_inactive: Color,
    pub color_border: Color,
    pub color_error: Color,
    pub color_success: Color,
    pub color_text: Color,
}

pub const PREDEFINED_THEMES: &[Theme] = &[
    Theme {
        name: "Sonus (Default)",
        color_accent: Color::Cyan,
        color_selected: Color::LightYellow,
        color_playing: Color::Green,
        color_inactive: Color::DarkGray,
        color_border: Color::Gray,
        color_error: Color::Red,
        color_success: Color::Green,
        color_text: Color::White,
    },
    Theme {
        name: "Dracula",
        color_accent: Color::Rgb(189, 147, 249),
        color_selected: Color::Rgb(255, 121, 198),
        color_playing: Color::Rgb(80, 250, 123),
        color_inactive: Color::Rgb(98, 114, 164),
        color_border: Color::Rgb(68, 71, 90),
        color_error: Color::Rgb(255, 85, 85),
        color_success: Color::Rgb(80, 250, 123),
        color_text: Color::Rgb(248, 248, 242),
    },
    Theme {
        name: "Dracula Darker",
        color_accent: Color::Rgb(139, 233, 253),
        color_selected: Color::Rgb(255, 121, 198),
        color_playing: Color::Rgb(80, 250, 123),
        color_inactive: Color::Rgb(98, 114, 164),
        color_border: Color::Rgb(40, 42, 54),
        color_error: Color::Rgb(255, 85, 85),
        color_success: Color::Rgb(80, 250, 123),
        color_text: Color::Rgb(248, 248, 242),
    },
    Theme {
        name: "Nord",
        color_accent: Color::Rgb(136, 192, 208),
        color_selected: Color::Rgb(143, 188, 187),
        color_playing: Color::Rgb(163, 190, 140),
        color_inactive: Color::Rgb(76, 86, 106),
        color_border: Color::Rgb(67, 76, 94),
        color_error: Color::Rgb(191, 97, 106),
        color_success: Color::Rgb(163, 190, 140),
        color_text: Color::Rgb(216, 222, 233),
    },
    Theme {
        name: "Nord Light",
        color_accent: Color::Rgb(94, 129, 172),
        color_selected: Color::Rgb(129, 161, 193),
        color_playing: Color::Rgb(163, 190, 140),
        color_inactive: Color::Rgb(216, 222, 233),
        color_border: Color::Rgb(229, 233, 240),
        color_error: Color::Rgb(191, 97, 106),
        color_success: Color::Rgb(163, 190, 140),
        color_text: Color::Rgb(46, 52, 64),
    },
    Theme {
        name: "Gruvbox Dark",
        color_accent: Color::Rgb(254, 128, 25),
        color_selected: Color::Rgb(250, 189, 47),
        color_playing: Color::Rgb(184, 187, 38),
        color_inactive: Color::Rgb(146, 131, 116),
        color_border: Color::Rgb(102, 92, 84),
        color_error: Color::Rgb(251, 73, 52),
        color_success: Color::Rgb(184, 187, 38),
        color_text: Color::Rgb(235, 219, 178),
    },
    Theme {
        name: "Gruvbox Light",
        color_accent: Color::Rgb(214, 93, 14),
        color_selected: Color::Rgb(181, 118, 20),
        color_playing: Color::Rgb(152, 151, 26),
        color_inactive: Color::Rgb(146, 131, 116),
        color_border: Color::Rgb(189, 174, 147),
        color_error: Color::Rgb(204, 36, 29),
        color_success: Color::Rgb(152, 151, 26),
        color_text: Color::Rgb(60, 56, 54),
    },
    Theme {
        name: "Rose Pine",
        color_accent: Color::Rgb(235, 188, 186),
        color_selected: Color::Rgb(246, 193, 119),
        color_playing: Color::Rgb(49, 116, 143),
        color_inactive: Color::Rgb(110, 106, 134),
        color_border: Color::Rgb(64, 61, 82),
        color_error: Color::Rgb(235, 111, 146),
        color_success: Color::Rgb(156, 207, 216),
        color_text: Color::Rgb(224, 222, 244),
    },
    Theme {
        name: "Rose Pine Moon",
        color_accent: Color::Rgb(196, 167, 231),
        color_selected: Color::Rgb(246, 193, 119),
        color_playing: Color::Rgb(49, 116, 143),
        color_inactive: Color::Rgb(137, 133, 163),
        color_border: Color::Rgb(86, 82, 110),
        color_error: Color::Rgb(235, 111, 146),
        color_success: Color::Rgb(156, 207, 216),
        color_text: Color::Rgb(224, 222, 244),
    },
    Theme {
        name: "Rose Pine Dawn",
        color_accent: Color::Rgb(144, 122, 169),
        color_selected: Color::Rgb(234, 157, 52),
        color_playing: Color::Rgb(40, 105, 131),
        color_inactive: Color::Rgb(152, 147, 178),
        color_border: Color::Rgb(223, 219, 217),
        color_error: Color::Rgb(180, 99, 122),
        color_success: Color::Rgb(86, 148, 159),
        color_text: Color::Rgb(87, 82, 121),
    },
    Theme {
        name: "Solarized Dark",
        color_accent: Color::Rgb(38, 139, 210),
        color_selected: Color::Rgb(42, 161, 152),
        color_playing: Color::Rgb(133, 153, 0),
        color_inactive: Color::Rgb(88, 110, 117),
        color_border: Color::Rgb(7, 54, 66),
        color_error: Color::Rgb(220, 50, 47),
        color_success: Color::Rgb(133, 153, 0),
        color_text: Color::Rgb(131, 148, 150),
    },
    Theme {
        name: "Solarized Light",
        color_accent: Color::Rgb(38, 139, 210),
        color_selected: Color::Rgb(211, 54, 130),
        color_playing: Color::Rgb(133, 153, 0),
        color_inactive: Color::Rgb(147, 161, 161),
        color_border: Color::Rgb(238, 232, 213),
        color_error: Color::Rgb(220, 50, 47),
        color_success: Color::Rgb(133, 153, 0),
        color_text: Color::Rgb(101, 123, 131),
    },
    Theme {
        name: "Monokai",
        color_accent: Color::Rgb(249, 38, 114),
        color_selected: Color::Rgb(102, 217, 239),
        color_playing: Color::Rgb(166, 226, 46),
        color_inactive: Color::Rgb(117, 113, 94),
        color_border: Color::Rgb(73, 72, 62),
        color_error: Color::Rgb(249, 38, 114),
        color_success: Color::Rgb(166, 226, 46),
        color_text: Color::Rgb(248, 248, 242),
    },
    Theme {
        name: "Matrix / Forest",
        color_accent: Color::Rgb(0, 255, 0),
        color_selected: Color::Rgb(0, 200, 0),
        color_playing: Color::Rgb(0, 255, 0),
        color_inactive: Color::Rgb(0, 95, 0),
        color_border: Color::Rgb(0, 95, 0),
        color_error: Color::Red,
        color_success: Color::Green,
        color_text: Color::Rgb(0, 255, 0),
    },
    Theme {
        name: "Catppuccin Mocha",
        color_accent: Color::Rgb(180, 190, 254),
        color_selected: Color::Rgb(245, 194, 231),
        color_playing: Color::Rgb(166, 227, 161),
        color_inactive: Color::Rgb(108, 112, 134),
        color_border: Color::Rgb(49, 50, 68),
        color_error: Color::Rgb(243, 139, 168),
        color_success: Color::Rgb(166, 227, 161),
        color_text: Color::Rgb(205, 214, 244),
    },
    Theme {
        name: "Catppuccin Macchiato",
        color_accent: Color::Rgb(183, 189, 248),
        color_selected: Color::Rgb(245, 189, 230),
        color_playing: Color::Rgb(166, 218, 149),
        color_inactive: Color::Rgb(110, 115, 141),
        color_border: Color::Rgb(54, 58, 79),
        color_error: Color::Rgb(237, 135, 150),
        color_success: Color::Rgb(166, 218, 149),
        color_text: Color::Rgb(202, 211, 245),
    },
    Theme {
        name: "Catppuccin Frappe",
        color_accent: Color::Rgb(186, 187, 254),
        color_selected: Color::Rgb(244, 184, 228),
        color_playing: Color::Rgb(166, 209, 137),
        color_inactive: Color::Rgb(115, 121, 148),
        color_border: Color::Rgb(65, 69, 89),
        color_error: Color::Rgb(231, 130, 132),
        color_success: Color::Rgb(166, 209, 137),
        color_text: Color::Rgb(198, 208, 245),
    },
    Theme {
        name: "Catppuccin Latte",
        color_accent: Color::Rgb(114, 135, 253),
        color_selected: Color::Rgb(234, 118, 203),
        color_playing: Color::Rgb(64, 160, 43),
        color_inactive: Color::Rgb(156, 160, 176),
        color_border: Color::Rgb(204, 208, 218),
        color_error: Color::Rgb(210, 15, 57),
        color_success: Color::Rgb(64, 160, 43),
        color_text: Color::Rgb(76, 79, 105),
    },
    Theme {
        name: "Tokyo Night",
        color_accent: Color::Rgb(115, 218, 202),
        color_selected: Color::Rgb(187, 154, 247),
        color_playing: Color::Rgb(158, 206, 106),
        color_inactive: Color::Rgb(86, 95, 137),
        color_border: Color::Rgb(56, 62, 90),
        color_error: Color::Rgb(247, 118, 142),
        color_success: Color::Rgb(158, 206, 106),
        color_text: Color::Rgb(169, 177, 214),
    },
    Theme {
        name: "Tokyo Night Storm",
        color_accent: Color::Rgb(122, 162, 247),
        color_selected: Color::Rgb(255, 158, 100),
        color_playing: Color::Rgb(158, 206, 106),
        color_inactive: Color::Rgb(86, 95, 137),
        color_border: Color::Rgb(47, 53, 77),
        color_error: Color::Rgb(247, 118, 142),
        color_success: Color::Rgb(158, 206, 106),
        color_text: Color::Rgb(169, 177, 214),
    },
    Theme {
        name: "One Dark",
        color_accent: Color::Rgb(97, 175, 239),
        color_selected: Color::Rgb(198, 120, 221),
        color_playing: Color::Rgb(152, 195, 121),
        color_inactive: Color::Rgb(92, 99, 112),
        color_border: Color::Rgb(75, 82, 99),
        color_error: Color::Rgb(224, 108, 117),
        color_success: Color::Rgb(152, 195, 121),
        color_text: Color::Rgb(171, 178, 191),
    },
    Theme {
        name: "One Light",
        color_accent: Color::Rgb(64, 120, 242),
        color_selected: Color::Rgb(166, 38, 164),
        color_playing: Color::Rgb(80, 161, 79),
        color_inactive: Color::Rgb(160, 161, 167),
        color_border: Color::Rgb(229, 229, 230),
        color_error: Color::Rgb(228, 86, 74),
        color_success: Color::Rgb(80, 161, 79),
        color_text: Color::Rgb(56, 58, 66),
    },
    Theme {
        name: "Ayu Dark",
        color_accent: Color::Rgb(255, 180, 84),
        color_selected: Color::Rgb(242, 159, 5),
        color_playing: Color::Rgb(170, 217, 76),
        color_inactive: Color::Rgb(92, 103, 115),
        color_border: Color::Rgb(28, 35, 43),
        color_error: Color::Rgb(240, 113, 120),
        color_success: Color::Rgb(170, 217, 76),
        color_text: Color::Rgb(179, 177, 173),
    },
    Theme {
        name: "Ayu Mirage",
        color_accent: Color::Rgb(255, 158, 59),
        color_selected: Color::Rgb(255, 203, 107),
        color_playing: Color::Rgb(170, 217, 76),
        color_inactive: Color::Rgb(92, 103, 115),
        color_border: Color::Rgb(36, 45, 56),
        color_error: Color::Rgb(240, 113, 120),
        color_success: Color::Rgb(170, 217, 76),
        color_text: Color::Rgb(204, 202, 194),
    },
];

pub fn color_to_string(color: Color) -> String {
    match color {
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "darkgray".to_string(),
        Color::LightRed => "lightred".to_string(),
        Color::LightGreen => "lightgreen".to_string(),
        Color::LightYellow => "lightyellow".to_string(),
        Color::LightBlue => "lightblue".to_string(),
        Color::LightMagenta => "lightmagenta".to_string(),
        Color::LightCyan => "lightcyan".to_string(),
        Color::White => "white".to_string(),
        Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        Color::Indexed(i) => format!("indexed({})", i),
        _ => "white".to_string(),
    }
}

pub fn update_theme(theme_name: &str) -> Result<(), String> {
    let theme = PREDEFINED_THEMES
        .iter()
        .find(|t| t.name.to_lowercase() == theme_name.to_lowercase())
        .ok_or_else(|| format!("Theme '{}' not found", theme_name))?;

    {
        let mut config = get_config().write().unwrap_or_else(|e| e.into_inner());
        config.color_accent = theme.color_accent;
        config.color_selected = theme.color_selected;
        config.color_playing = theme.color_playing;
        config.color_inactive = theme.color_inactive;
        config.color_border = theme.color_border;
        config.color_error = theme.color_error;
        config.color_success = theme.color_success;
        config.color_text = theme.color_text;
    }

    if let Some(config_dir) = dirs::config_dir().map(|p| p.join("sonus")) {
        let config_path = config_dir.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let keys = [
                ("color_accent", theme.color_accent),
                ("color_selected", theme.color_selected),
                ("color_playing", theme.color_playing),
                ("color_inactive", theme.color_inactive),
                ("color_border", theme.color_border),
                ("color_error", theme.color_error),
                ("color_success", theme.color_success),
                ("color_text", theme.color_text),
            ];

            for (key, color) in keys {
                let new_line = format!("{} = \"{}\"", key, color_to_string(color));
                let mut found = false;
                for line in &mut lines {
                    if matches_config_key(line.trim(), key) {
                        *line = new_line.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    lines.push(new_line);
                }
            }
            write_config_atomically(&config_path, &lines.join("\n"));
        }
    }

    Ok(())
}

