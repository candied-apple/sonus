# Sonus Project Audit Report

> Generated: 2026-06-22

---

## Table of Contents

1. [Current Project Structure](#1-current-project-structure)
2. [mod.rs Violations (Logic Inside mod.rs)](#2-modrs-violations-logic-inside-modrs)
3. [Dead Code & Unused Features](#3-dead-code--unused-features)
4. [Duplicated Code](#4-duplicated-code)
5. [Naming Issues](#5-naming-issues)
6. [Files Not Divided Per Feature](#6-files-not-divided-per-feature)
7. [Missing Features & Incomplete Implementations](#7-missing-features--incomplete-implementations)
8. [Recommendations](#8-recommendations)
9. [Proposed New Structure](#9-proposed-new-structure)
10. [AGENTS.md Update](#10-agentsmd-update)

---

## 1. Current Project Structure

```
sonus/
├── Cargo.toml                          # Workspace root
├── AGENTS.md
├── PKGBUILD
├── org.candied_apple.sonus.yaml        # Flatpak manifest
├── dist/                               # ⚠️ Build artifact (RPM) — should be gitignored
│   └── sonus.rpm
├── pkg/                                # makepkg staging — gitignored
├── sonus/                              # Bare git clone — makepkg artifact
├── src/sonus/                          # makepkg extraction — gitignored
├── sonus-0.2.0-1-x86_64.pkg.tar.zst   # Arch package artifact
│
├── crates/
│   ├── sonus-core/
│   │   ├── Cargo.toml
│   │   ├── examples/
│   │   │   └── inspect_watch.rs        # Debug/dev example
│   │   └── src/
│   │       ├── lib.rs                  # ✅ Only module declarations
│   │       ├── config.rs              # 628 lines — config + themes + color parsing
│   │       ├── log.rs                 # 49 lines — in-memory log ring buffer
│   │       ├── types.rs               # 67 lines — TrackItem, PlayerState, enums
│   │       ├── util.rs                # 62 lines — is_valid_video_id, parse_time_string, fit_to_width
│   │       ├── api/
│   │       │   ├── mod.rs             # ✅ Only module declaration
│   │       │   └── client.rs          # ~470 lines — YtmClient, LRCLib, HTTP, recommendations
│   │       ├── db/
│   │       │   ├── mod.rs             # ❌ Contains Db struct, new(), init(), migrations (138 lines)
│   │       │   ├── playlists.rs       # Playlist CRUD operations
│   │       │   └── tracks.rs          # Track DB operations (350 lines)
│   │       ├── mpris/
│   │       │   └── mod.rs             # ❌ Entire MPRIS implementation (233 lines, no sub-files)
│   │       └── player/
│   │           ├── mod.rs             # ❌ Contains PlayerCommand/PlayerEvent enums + spawn() (176 lines)
│   │           ├── cache.rs           # Audio cache management
│   │           ├── stream.rs          # yt-dlp streaming + decoding
│   │           └── volume.rs          # 1-line volume curve function
│
│   └── sonus-tui/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                # 61 lines — entry point
│           ├── app.rs                 # ❌ 988 lines — God Object (event loop, API results, LRC parsing, caching)
│           ├── config.rs              # 91 lines — Core→Ratatui color bridge
│           ├── log.rs                 # 17 lines — wrapper around core log
│           ├── util.rs                # 12 lines — thin wrapper around core util
│           ├── commands/              # ❌ Misleading name — these are NOT commands, they are App impl blocks
│           │   ├── mod.rs             # ✅ Only module declarations
│           │   ├── explore.rs         # 140 lines — explore/For You logic
│           │   ├── import.rs          # 246 lines — Spotify import
│           │   ├── mouse.rs           # 572 lines — mouse event handler
│           │   ├── navigation.rs      # 135 lines — sidebar, toggle panels
│           │   ├── palette.rs         # 653 lines — command palette actions
│           │   ├── playback.rs        # 674 lines — play, queue, volume, lyrics, MPRIS, recommendations
│           │   └── search.rs          # 54 lines — search key handler
│           ├── state/
│           │   ├── mod.rs             # ✅ Only module declarations
│           │   ├── app_state.rs       # 396 lines — AppState struct + index management
│           │   └── command_palette.rs # 123 lines — PaletteMode, ConfirmAction, AVAILABLE_COMMANDS
│           └── ui/
│               ├── mod.rs             # ❌ Contains render() + render_import_progress() (110 lines)
│               ├── command_palette.rs # Command palette UI rendering
│               ├── components.rs      # format_duration, ensure_scroll helpers
│               ├── header.rs          # Header bar rendering
│               ├── help.rs            # Help screen rendering
│               ├── layout.rs          # Layout calculation
│               ├── lyrics.rs          # Lyrics panel rendering
│               ├── now_playing.rs     # Now playing / footer rendering
│               ├── queue.rs           # Queue panel rendering
│               ├── sidebar.rs         # Sidebar rendering
│               └── tracklist.rs       # Track list rendering (album header, dual-box)
```

---

## 2. mod.rs Violations (Logic Inside mod.rs)

Per the requirement: **mod.rs files should only be used for imports, no code inside**.

| File | Lines | Violation |
|------|-------|-----------|
| `sonus-core/src/db/mod.rs` | 138 | Contains `Db` struct, `new()`, `init()`, full schema definitions, and migration logic |
| `sonus-core/src/player/mod.rs` | 176 | Contains `PlayerCommand` + `PlayerEvent` enums and the entire `spawn()` function with the player event loop |
| `sonus-core/src/mpris/mod.rs` | 233 | Contains the **entire MPRIS implementation** — `MprisState`, `MprisCommand`, `MprisSignal`, `spawn()`, `serve()`, platform-specific code |
| `sonus-tui/src/ui/mod.rs` | 110 | Contains `render()` dispatch function and `render_import_progress()` function |

### What Should Be Done

| Current | Move To |
|---------|---------|
| `db/mod.rs` → `Db` struct, `new()`, `init()` | `db/connection.rs` (connection + schema) |
| `db/mod.rs` → migrations | `db/migration.rs` |
| `player/mod.rs` → enums | `player/command.rs` |
| `player/mod.rs` → `spawn()` | `player/engine.rs` |
| `mpris/mod.rs` → entire file | Split into `mpris/state.rs`, `mpris/handler.rs`, `mpris/platform.rs` |
| `ui/mod.rs` → `render()`, `render_import_progress()` | `ui/renderer.rs` |

---

## 3. Dead Code & Unused Features

### Confirmed Dead Code

| Location | Item | Evidence |
|----------|------|----------|
| `sonus-tui/src/log.rs:8-11` | `pub fn entries()` | Marked `#[allow(dead_code)]` — wrapper never called |
| `sonus-tui/src/log.rs:13-16` | `pub fn clear()` | Marked `#[allow(dead_code)]` — wrapper never called |
| `sonus-core/src/config.rs:32` | `color_error` field in `Config` | Parsed from config file but **never read** via any accessor function — no `color_error()` getter exists |
| `sonus-core/src/config.rs:278` | `color_error` field in `Theme` | Same — set in every theme but never consumed |
| `sonus-core/src/log.rs:34` | `pub fn clear()` | Only called from TUI's dead-code wrapper |
| `sonus-tui/src/state/app_state.rs:66` | `history_index` field | Initialized to 0, never modified or read outside `Default` |
| `sonus-tui/src/util.rs:1-3` | `is_valid_video_id()` wrapper | Thin wrapper that just calls `sonus_core::util::is_valid_video_id()` — could be removed; callers should use core directly |
| `sonus-tui/src/util.rs:5-7` | `parse_time_string()` wrapper | Same — thin wrapper around core |
| `sonus-tui/src/util.rs:9-11` | `fit_to_width()` wrapper | Same — converts `Cow<str>` to owned `String`, losing the zero-copy benefit |

### Potentially Unused

| Location | Item | Notes |
|----------|------|-------|
| `sonus-core/examples/inspect_watch.rs` | Entire example | Dev/debug example — may still be useful for development but not part of the app |
| `sonus-tui/Cargo.toml` | `serde` dependency | No direct `Serialize`/`Deserialize` derives in sonus-tui; only needed transitively via `serde_json` |

---

## 4. Duplicated Code

### 4.1 Duplicated Play/Pause/Stop Logic

The play/pause/stop toggle logic is written **4 separate times** in nearly identical form:

1. **`app.rs:807-821`** — Keyboard `Space` handler
2. **`commands/palette.rs:257-270`** — Command palette "playback: play/pause"
3. **`commands/playback.rs:566-596`** — MPRIS `PlayPause` handler
4. **`commands/mouse.rs:404-417`** — Mouse click on play button

Each duplicates:
```rust
match self.state.player.status {
    PlayStatus::Playing => { send Pause; set Paused }
    PlayStatus::Paused => { send Resume; set Playing }
    PlayStatus::Stopped => { play_selected_track() }
}
```

> **Fix**: Extract a `toggle_play_pause(&mut self, player_cmd_tx)` method.

### 4.2 Duplicated Stop Logic

Full player stop (clear track, cover, position) is written **3 times**:

1. `commands/palette.rs:272-281` — Command palette "playback: stop"
2. `commands/playback.rs:598-607` — MPRIS stop handler
3. `commands/playback.rs:191-201` — `play_next_in_queue()` when queue is empty

> **Fix**: Extract a `stop_playback(&mut self)` method.

### 4.3 Duplicated Repeat-One TrackItem Reconstruction

The pattern of reconstructing a `TrackItem` from the current player state is duplicated in:

1. `commands/playback.rs:161-181` — `play_next_in_queue()` Repeat::One
2. `commands/playback.rs:227-247` — `play_previous_in_queue()` Repeat::One
3. `commands/playback.rs:282-299` — `play_previous_in_queue()` fallback

All three create a TrackItem with `index: 0`, `album: None`, `category: Song`, hardcoded fields.

> **Fix**: Extract a `current_track_item(&self) -> Option<TrackItem>` method.

### 4.4 Duplicated Source List Resolution

The pattern for resolving the "active source list" based on `active_page` and `explore_section` is written **4 times** in `app_state.rs`:

1. `active_track_global_index()` (L140-149)
2. `active_track_count()` (L192-201)
3. `active_track()` (L281-290)
4. `active_track_list()` (L296-305)

Each has the same match statement:
```rust
let source_list = match self.active_page {
    ActivePage::Library => &self.tracks,
    ActivePage::Search => &self.tracks,
    ActivePage::Explore => match self.explore_section {
        ExploreSection::ForYou => &self.explore_for_you,
        ExploreSection::History => &self.history,
        ExploreSection::TopArtists => &self.tracks,
    },
};
let source_list = if self.is_search_results() { &self.tracks } else { source_list };
```

> **Fix**: Extract a `fn source_list(&self) -> &[Arc<TrackItem>]` helper.

### 4.5 Duplicated Shuffle/Repeat Toggle

Shuffle toggle appears in 3 places: keyboard `s`, command palette, and mouse click.
Repeat cycle appears in 3 places: keyboard `r`, command palette, and mouse click.

> **Fix**: Methods `toggle_shuffle()` and `cycle_repeat_mode()` already partially exist in spirit but aren't factored out.

### 4.6 Duplicated Color Bridge (sonus-tui/src/config.rs)

The entire TUI `config.rs` is a manual bridge mapping `sonus_core::config::Color` → `ratatui::style::Color`. There are 19 color variants mapped one-by-one. This could be a `From` implementation on either side.

### 4.7 Duplicated Utility Wrappers (sonus-tui/src/util.rs)

All 3 functions are trivial wrappers around `sonus_core::util::*`. The TUI crate should just use `sonus_core::util::*` directly — there's no added value.

---

## 5. Naming Issues

### 5.1 `commands/` Folder — Misleading Name

**Problem**: The `commands/` folder does **not** contain "commands" in any traditional sense (no Command pattern, no command structs, no command dispatching). It contains **`impl App` blocks** split by feature area.

**Current files and what they actually are**:

| File | Actual Content |
|------|---------------|
| `commands/explore.rs` | App methods for explore/For-You feature |
| `commands/import.rs` | Spotify import logic |
| `commands/mouse.rs` | Mouse event handling |
| `commands/navigation.rs` | Sidebar activation, panel toggling |
| `commands/palette.rs` | Command palette logic |
| `commands/playback.rs` | Playback, queue, lyrics, MPRIS, volume, cover art |
| `commands/search.rs` | Search key handling |

**Additionally confusing**: The naming `commands/palette.rs` clashes with `state/command_palette.rs` and `ui/command_palette.rs`. Three files named around "command palette" in three different folders.

**Recommended rename**: `commands/` → `handlers/` or `actions/` — these are event handlers / action implementations, not "commands" in the Command pattern sense.

### 5.2 File Content Doesn't Match Name

| Current Name | Problem | Suggested Name |
|---|---|---|
| `commands/playback.rs` | Contains playback + queue + lyrics + MPRIS + volume + cover art + recommendations (674 lines) | Should be split into separate files |
| `commands/navigation.rs` | Contains sidebar + panel toggles | `handlers/sidebar.rs` + inline panel toggles |
| `commands/mouse.rs` | Mouse event handling split from keyboard in app.rs | `handlers/mouse.rs` |
| `state/command_palette.rs` | Defines palette modes, actions, available commands | `state/palette.rs` |

### 5.3 Inconsistent Naming Patterns

- `SidebarItem` lives in `state/app_state.rs` but is sidebar-specific
- `SpotifyImportState` lives in `state/app_state.rs` but is import-specific
- `ExploreSection`, `ActivePage`, `SearchTab` all in `app_state.rs` — these are navigation concerns mixed with app state

---

## 6. Files Not Divided Per Feature

### 6.1 `app.rs` — God Object (988 lines)

`app.rs` contains **too many responsibilities**:

| Lines | Responsibility | Should Be |
|---|---|---|
| 1-48 | `ApiResult` enum, `App` struct definition | `app.rs` (keep) |
| 50-97 | `App::new()` — DB init, migration | `app.rs` (keep) |
| 100-444 | `App::run()` — main event loop, result polling | `app.rs` (keep, but extract result handling) |
| 447-482 | `handle_player_event()` | `handlers/player_event.rs` |
| 485-870 | `handle_key()` — keyboard event dispatching | `handlers/keyboard.rs` |
| 873-944 | `strip_inline_timestamps()`, `parse_lrc()`, `parse_lrc_timestamp()` | `lrc.rs` (LRC parsing) |
| 947-986 | `for_you_cache_path()`, `save_for_you_cache()`, `load_for_you_cache()` | `cache.rs` (For-You cache) |

### 6.2 `commands/playback.rs` — Multiple Features in One File (674 lines)

This file contains **6 unrelated features**:

| Feature | Lines | Should Be |
|---|---|---|
| Track playback (play_track_item, play_selected_track, etc.) | ~150 | `handlers/playback.rs` |
| Queue management (play_from_queue, play_next, play_previous, add_to_queue) | ~150 | `handlers/queue.rs` |
| Volume control | ~10 | `handlers/volume.rs` or inline |
| Cover art fetching | ~40 | `handlers/cover.rs` |
| Lyrics fetching | ~120 | `handlers/lyrics.rs` |
| MPRIS state management | ~100 | `handlers/media_controls.rs` |
| Recommendations | ~20 | `handlers/recommendations.rs` or in explore |

### 6.3 `commands/palette.rs` — Massive Monolith (653 lines)

Contains all palette logic: open, close, filter, key handling, **and** all action execution (create playlist, delete playlist, import, seek, theme change, context actions for every focus).

Should be split:
- `handlers/palette.rs` — open/close/filter/key handling
- Context action execution should route to existing feature methods

### 6.4 `commands/mouse.rs` — 572 Lines of Coordinate Math

All mouse handling is in one giant file. Could be split by area:
- `handlers/mouse.rs` — dispatch + common helpers (`rect_contains`, `get_inner_rect`)
- Or keep as-is since mouse handling is inherently one event type

### 6.5 `sonus-core/src/config.rs` — 628 Lines

Contains 3 distinct features:
1. Config struct + parsing + load/save (lines 1-268)
2. Theme definitions (lines 270-548, ~20 predefined themes)
3. Theme update logic (lines 550-628)

Should be:
- `config/settings.rs` — Config struct, parsing, persistence
- `config/theme.rs` — Theme struct, predefined themes, theme application
- `config/color.rs` — Color enum, parse_color(), color_to_string()

### 6.6 `sonus-core/src/api/client.rs` — Multiple API Clients in One File

Contains:
1. Shared HTTP client (`shared_http_client`)
2. YouTube Music search/recommendations (`YtmClient`)
3. LRCLib lyrics API
4. GitHub release check API
5. YouTube Music lyrics API

Should be:
- `api/http.rs` — shared HTTP client
- `api/ytm.rs` — YouTube Music client
- `api/lyrics.rs` — LRCLib + YTM lyrics
- `api/github.rs` — version check

### 6.7 `sonus-core/src/db/tracks.rs` — 350 Lines, Multiple Concerns

Contains playlist track operations, history operations, album lookups, AND lyrics caching — all in one file.

Should be:
- `db/playlist_tracks.rs` — playlist track CRUD
- `db/history.rs` — history operations
- `db/lyrics_cache.rs` — lyrics cache operations

---

## 7. Missing Features & Incomplete Implementations

### Undocumented Features (Missing from README)
| Feature | Present in Code | In README |
|---|---|---|
| MPRIS / Media Controls | ✅ `mpris/mod.rs` | ❌ |
| Spotify Playlist Import | ✅ `commands/import.rs` | ❌ |
| Cover Art Display | ✅ `ratatui-image` integration | ❌ |
| Personalized Recommendations (For You) | ✅ `commands/explore.rs` | ❌ |
| Theme System (20+ themes) | ✅ `config.rs` themes | ❌ |
| Command Palette | ✅ Full implementation | ❌ |

### Missing `.gitignore` Entries
- `/dist/` — contains `sonus.rpm` (5.3 MB build artifact tracked in git)

### Config Feature: `color_error` — Dead
- Defined in `Config` struct, set in all 20+ themes, but **no getter function exists**.
- No UI code ever reads `color_error`. It is fully dead.

### Potential Missing Features
| Feature | Status |
|---|---|
| Tests | ❌ No tests exist in either crate (acknowledged in AGENTS.md) |
| Equalizer / Audio Effects | ❌ Not implemented |
| Playlist reorder / drag | ❌ Not implemented |
| Playlist rename | ❌ Not implemented |
| Export playlist | ❌ Not implemented |
| Keyboard shortcut customization | ❌ Hardcoded keys |
| Sleep timer | ❌ Not implemented |

---

## 8. Recommendations

### Priority 1 — Critical Structural Issues

1. **Rename `commands/` → `handlers/`** — The name "commands" is misleading. These files contain `impl App` blocks for handling events, not Command pattern implementations. The name "commands" would be appropriate for what's currently in `state/command_palette.rs`.

2. **Empty out all `mod.rs` files** — Move logic out of `db/mod.rs`, `player/mod.rs`, `mpris/mod.rs`, and `ui/mod.rs` into properly named sub-files. Keep only `pub mod` declarations and re-exports.

3. **Split `app.rs`** — Extract LRC parsing, For-You caching, and move `handle_key()` to the handlers module.

4. **Split `commands/playback.rs`** — This 674-line file handles 6 unrelated features. Split by feature.

### Priority 2 — Code Quality

5. **Extract duplicated play/pause/stop/repeat logic** — Create shared methods (`toggle_play_pause()`, `stop_playback()`, `current_track_item()`, `source_list()`).

6. **Delete `sonus-tui/src/util.rs`** — It's just thin wrappers. Use `sonus_core::util::*` directly.

7. **Delete dead code** — Remove `history_index`, dead `log` wrapper functions, and either implement `color_error` or remove it from Config/Theme.

8. **Split `sonus-core/src/config.rs`** into `config/settings.rs`, `config/theme.rs`, `config/color.rs`.

9. **Split `sonus-core/src/api/client.rs`** into separate API client files.

### Priority 3 — Naming & Consistency

10. **Resolve `command_palette` naming collision** — Three files named around "command palette" across `commands/palette.rs`, `state/command_palette.rs`, `ui/command_palette.rs`. Rename `commands/palette.rs` → `handlers/palette.rs`, rename `state/command_palette.rs` → `state/palette.rs`.

11. **Add `/dist/` to `.gitignore`** — The RPM build artifact should not be tracked.

12. **Document undocumented features in README** — MPRIS, Spotify import, cover art, For You recommendations, theme system, command palette.

---

## 9. Proposed New Structure

```
crates/
├── sonus-core/
│   └── src/
│       ├── lib.rs                    # Module declarations only
│       ├── types.rs                  # TrackItem, PlayerState, enums (keep as-is)
│       │
│       ├── config/
│       │   ├── mod.rs                # pub mod + re-exports only
│       │   ├── settings.rs           # Config struct, load, save, parse
│       │   ├── theme.rs              # Theme struct, PREDEFINED_THEMES, update_theme()
│       │   └── color.rs              # Color enum, parse_color(), color_to_string()
│       │
│       ├── api/
│       │   ├── mod.rs                # pub mod + re-exports only
│       │   ├── http.rs               # shared_http_client()
│       │   ├── ytm.rs                # YtmClient — search, recommendations, playlist
│       │   ├── lyrics.rs             # LRCLib + YTM lyrics APIs
│       │   └── version.rs            # check_latest_release()
│       │
│       ├── db/
│       │   ├── mod.rs                # pub mod + re-exports only
│       │   ├── connection.rs         # Db struct, new(), pragmas
│       │   ├── migration.rs          # init(), schema creation, column migrations
│       │   ├── playlists.rs          # Playlist CRUD (keep as-is)
│       │   ├── history.rs            # History track operations
│       │   ├── tracks.rs             # Playlist track operations
│       │   └── lyrics_cache.rs       # Lyrics cache operations
│       │
│       ├── player/
│       │   ├── mod.rs                # pub mod + re-exports only
│       │   ├── command.rs            # PlayerCommand, PlayerEvent enums
│       │   ├── engine.rs             # spawn(), player event loop
│       │   ├── stream.rs             # yt-dlp streaming + decoding (keep as-is)
│       │   ├── cache.rs              # Audio cache management (keep as-is)
│       │   └── volume.rs             # Volume curve (keep as-is)
│       │
│       ├── mpris/
│       │   ├── mod.rs                # pub mod + re-exports only
│       │   ├── state.rs              # MprisState struct
│       │   ├── command.rs            # MprisCommand, MprisSignal enums
│       │   └── server.rs             # spawn(), serve(), platform helpers
│       │
│       ├── log.rs                    # Keep as-is (small and focused)
│       └── util.rs                   # Keep as-is (small and focused)
│
└── sonus-tui/
    └── src/
        ├── main.rs                   # Entry point (keep as-is)
        ├── app.rs                    # App struct + run() event loop only
        ├── config.rs                 # Color bridge (keep as-is)
        │
        ├── handlers/                 # ← renamed from "commands/"
        │   ├── mod.rs                # pub mod declarations only
        │   ├── keyboard.rs           # handle_key() — extracted from app.rs
        │   ├── mouse.rs              # Mouse event handling (keep content)
        │   ├── player_event.rs       # handle_player_event() — extracted from app.rs
        │   ├── playback.rs           # play_track_item, play_selected, play_from_queue
        │   ├── queue.rs              # Queue management (next, previous, add, shuffle)
        │   ├── lyrics.rs             # fetch_lyrics, LRC parsing (extracted from app.rs)
        │   ├── cover.rs              # fetch_cover_image
        │   ├── media_controls.rs     # MPRIS state update + command handling
        │   ├── search.rs             # Search key handling (keep content)
        │   ├── explore.rs            # For You / explore data loading
        │   ├── navigation.rs         # Sidebar activation, panel toggles (keep content)
        │   ├── palette.rs            # Command palette open/close/filter/key/execute
        │   └── import.rs             # Spotify import (keep content)
        │
        ├── state/
        │   ├── mod.rs                # pub mod declarations only
        │   ├── app_state.rs          # AppState struct + helper methods
        │   └── palette.rs            # ← renamed from command_palette.rs
        │
        ├── ui/
        │   ├── mod.rs                # pub mod declarations only
        │   ├── renderer.rs           # render() dispatch + render_import_progress()
        │   ├── command_palette.rs    # Palette UI rendering
        │   ├── components.rs         # Shared UI components
        │   ├── header.rs             # Header rendering
        │   ├── help.rs               # Help screen rendering
        │   ├── layout.rs             # Layout calculation
        │   ├── lyrics.rs             # Lyrics panel rendering
        │   ├── now_playing.rs        # Footer / now playing
        │   ├── queue.rs              # Queue panel rendering
        │   ├── sidebar.rs            # Sidebar rendering
        │   └── tracklist.rs          # Track list rendering
        │
        └── lrc.rs                    # LRC parsing (extracted from app.rs)
```

### Key Changes Summary

| Change | Rationale |
|---|---|
| `commands/` → `handlers/` | Correct semantic — these handle events, not issue commands |
| `db/mod.rs` logic → `db/connection.rs` + `db/migration.rs` | mod.rs should be imports only |
| `player/mod.rs` logic → `player/command.rs` + `player/engine.rs` | mod.rs should be imports only |
| `mpris/mod.rs` → `mpris/state.rs` + `mpris/command.rs` + `mpris/server.rs` | mod.rs should be imports only |
| `ui/mod.rs` logic → `ui/renderer.rs` | mod.rs should be imports only |
| `app.rs` parse_lrc → `lrc.rs` | Feature separation |
| `app.rs` for_you_cache → extracted to appropriate handler | Feature separation |
| `playback.rs` split → `playback.rs`, `queue.rs`, `lyrics.rs`, `cover.rs`, `media_controls.rs` | Feature separation |
| `state/command_palette.rs` → `state/palette.rs` | Reduce naming collision |
| `config.rs` → `config/settings.rs` + `config/theme.rs` + `config/color.rs` | Feature separation |
| `api/client.rs` → `api/ytm.rs` + `api/lyrics.rs` + `api/version.rs` + `api/http.rs` | Feature separation |
| Delete `sonus-tui/src/util.rs` | Dead thin wrappers |
| Delete `sonus-tui/src/log.rs` dead fns | Dead code |

---

## 10. AGENTS.md Update

The following updates should be made to AGENTS.md:

1. **Add note about `handlers/` folder** (after rename) — that it contains `impl App` blocks split by feature, not a Command pattern
2. **Add note about the `lrc.rs` file** — LRC timestamp parsing, strip_inline_timestamps
3. **Document the config module split** — settings/theme/color separation
4. **Add note about `color_error` removal** or implementation
5. **Update file path references** — if paths in AGENTS.md reference `commands/`, update to `handlers/`
6. **Add note about `dist/` directory** — should be gitignored, is a build artifact
