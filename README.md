# 🎵 Sonus

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg?style=flat-downright&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-downright)](https://opensource.org/licenses/MIT)

**Sonus** is a fast, lightweight, and highly customizable Terminal User Interface (TUI) music player for **YouTube Music**, built entirely in Rust using [Ratatui](https://github.com/ratatui/ratatui). It streams audio directly to your output device with low latency, supports offline caching, displays synced lyrics via LRCLib, and features a broad collection of beautiful built-in themes.

---

## ✨ Features

- 🎧 **Direct Streaming**: Stream audio in real-time from YouTube Music with dual-process buffering for ultra-low playback latency.
- ⚡ **Offline Caching**: Automatically cache tracks locally with automatic Least Recently Used (LRU) cache cleaning.
- 🎤 **Lyrics Integration**: Fetch and display synchronized lyrics directly from [LRCLib](https://lrclib.net/).
- 🎨 **Beautiful Built-in Themes**: Select from popular color schemes like Catppuccin, Tokyo Night, Rose Pine, Nord, Gruvbox, Dracula, One Dark, and more.
- 🎛️ **Command Palette**: Run actions dynamically with an in-app command palette (`:` or `Ctrl+P`).
- 📂 **Local Database & Playlists**: Manage playlists, track playback history, and queue songs locally via SQLite.
- 🖥️ **Responsive Layout**: Rearrange panel widths on-the-fly with interactive resizing controls.

---

## 📋 Requirements

Before running Sonus, ensure the following dependencies are installed and available on your system `PATH`:

- **Rust**: [Rustup](https://rustup.rs/) (edition 2024 compiler).
- **yt-dlp**: Required for fetching and parsing YouTube audio streams.
- **ALSA / Audio libraries**: If compiling on Linux, make sure development packages for your audio server are installed (e.g., `libasound2-dev` on Debian/Ubuntu).

---

## 🚀 Installation

Build and install from the source directory:

```bash
# Clone the repository
git clone https://github.com/candied-apple/sonus.git
cd sonus

# Build the release binary
cargo build --release

# Run the player
./target/release/sonus
```

---

## ⌨️ Keyboard Shortcuts

Sonus can be fully navigated and controlled via the keyboard:

### Navigation
- <kbd>Tab</kbd> / <kbd>Shift + Tab</kbd>: Cycle focus through active panels
- <kbd>/</kbd>: Focus the search bar
- <kbd>↑</kbd> / <kbd>↓</kbd>: Navigate through lists / tracks
- <kbd>Enter</kbd>: Select item / Play track
- <kbd>Esc</kbd>: Close menus / Clear search focus

### Playback Controls
- <kbd>Space</kbd>: Play / Pause
- <kbd>n</kbd> / <kbd>p</kbd>: Next / Previous track
- <kbd>s</kbd>: Toggle shuffle mode
- <kbd>r</kbd>: Cycle repeat mode
- <kbd>+</kbd> / <kbd>-</kbd>: Adjust volume

### Panels & Views
- <kbd>q</kbd>: Toggle Queue panel
- <kbd>h</kbd>: Toggle History panel
- <kbd>l</kbd>: Toggle Lyrics panel
- <kbd>?</kbd>: Toggle Help overlay

### Resizing Panel Widths
- <kbd>Ctrl + R</kbd>: Toggle resize mode
- <kbd>←</kbd> / <kbd>→</kbd>: Adjust width of the focused panel (when in resize mode)

### Command Palette
- <kbd>:</kbd> / <kbd>Ctrl + P</kbd>: Open Command Palette (type command names and press <kbd>Enter</kbd>)

### Context Menus
- <kbd>c</kbd>: Open track context menu
- <kbd>↑</kbd> / <kbd>↓</kbd>: Move selection
- <kbd>Enter</kbd>: Execute menu item
- <kbd>Esc</kbd>: Close context menu

### General
- <kbd>Ctrl + C</kbd>: Force quit the application

---

## ⚙️ Configuration

Sonus reads its configuration from `~/.config/sonus/config.toml`. The configuration is initialized automatically with default values on the first run.

```toml
# ~/.config/sonus/config.toml

# Use Nerd Font icons (set to false if your terminal doesn't support Nerd Fonts)
use_nerd_font = true

# The default volume level when player starts (between 0.0 and 1.0)
default_volume = 0.8

# The audio cache limit in megabytes (MB)
cache_limit_mb = 1000

# The maximum number of history songs to keep in playback history
history_limit = 100

# Themes & Colors (Hex codes e.g. "#00FFFF" or standard color names)
color_accent = "cyan"
color_selected = "lightyellow"
color_playing = "green"
color_inactive = "darkgray"
color_border = "gray"
color_error = "red"
color_success = "green"
color_text = "white"
```

*Note: You can override the Nerd Fonts setting dynamically by running the application with the `SONUS_NO_NERD=1` environment variable.*

---

## 🎨 Predefined Themes

You can change the theme of the application using the Command Palette (`:` or `Ctrl+P`) and running the command matching your theme name. Available themes out of the box:

- **Sonus (Default)**
- **Dracula** / **Dracula Darker**
- **Nord** / **Nord Light**
- **Gruvbox Dark** / **Gruvbox Light**
- **Rose Pine** / **Rose Pine Moon** / **Rose Pine Dawn**
- **Solarized Dark** / **Solarized Light**
- **Monokai**
- **Matrix / Forest**
- **Catppuccin Mocha** / **Catppuccin Macchiato** / **Catppuccin Frappe** / **Catppuccin Latte**
- **Tokyo Night** / **Tokyo Night Storm**
- **One Dark** / **One Light**
- **Ayu Dark** / **Ayu Mirage**

---

## 🛠️ Project Structure

The project is structured as a Cargo workspace with two main crates:

- **[`crates/sonus-core`](file:///home/alp/Documents/Sonus/crates/sonus-core)**: Backend engine responsible for yt-dlp integrations, Symphonia decoders, rodio audio player sinks, caching logic, config handling, and SQLite database schema administration.
- **[`crates/sonus-tui`](file:///home/alp/Documents/Sonus/crates/sonus-tui)**: Terminal UI built with Ratatui, handling user interaction, layout renders, event handling loop, and state management.

---

## 📄 License

This project is licensed under the MIT License. See the [LICENSE](file:///home/alp/Documents/Sonus/LICENSE) file for details.
