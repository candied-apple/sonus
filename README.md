# Sonus

A TUI music player for YouTube Music, written in Rust using Ratatui. Streams audio, caches tracks offline, shows synced lyrics via LRCLib, and ships with a bunch of themes.

## Requirements

- Rust (2024 edition)
- `yt-dlp` on PATH
- ALSA dev libraries (Linux; e.g. `libasound2-dev`)

## Installation

```
git clone https://github.com/candied-apple/sonus.git
cd sonus
cargo build --release
./target/release/sonus
```

## Usage

Keybindings are listed in-app via `?`. The most important ones:

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle focus |
| `/` | Search bar |
| `Space` | Play / Pause |
| `n` / `p` | Next / Previous |
| `s` / `r` | Shuffle / Repeat |
| `+` / `-` | Volume |
| `q` / `h` / `l` | Queue / History / Lyrics panel |
| `:` / `Ctrl+P` | Command palette |
| `c` | Context menu |
| `Ctrl+R` | Resize mode |
| `Ctrl+C` | Quit |

## Configuration

Reads `~/.config/sonus/config.toml` (created with defaults on first run):

```toml
use_nerd_font = true
default_volume = 0.8
cache_limit_mb = 1000
history_limit = 100
color_accent = "cyan"
color_selected = "lightyellow"
color_playing = "green"
color_inactive = "darkgray"
color_border = "gray"
color_error = "red"
color_success = "green"
color_text = "white"
```

Override Nerd Fonts at runtime with `SONUS_NO_NERD=1`.

## Themes

Change theme via the command palette. Available:

Sonus (Default), Dracula, Dracula Darker, Nord, Nord Light, Gruvbox Dark, Gruvbox Light, Rose Pine, Rose Pine Moon, Rose Pine Dawn, Solarized Dark, Solarized Light, Monokai, Matrix, Forest, Catppuccin Mocha/Macchiato/Frappe/Latte, Tokyo Night, Tokyo Night Storm, One Dark, One Light, Ayu Dark, Ayu Mirage.

## Project structure

```
crates/
  sonus-core/   -- backend: yt-dlp, audio decode, caching, config, SQLite
  sonus-tui/    -- frontend: Ratatui UI, event loop, state
```

## License

MIT
