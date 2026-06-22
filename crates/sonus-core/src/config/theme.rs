use super::color::{Color, color_to_string};
use super::settings::{get_config, write_config_atomically, matches_config_key};

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
