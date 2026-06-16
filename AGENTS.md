# AGENTS.md

## Must-follow constraints
- `yt-dlp` must be on system PATH (runtime binary dependency).
- Do not add YouTube Music authentication; the unauthenticated innertube client (`NoAuthToken`) is required.
- Do not add a `toml` parsing crate; `~/.config/sonus/config.toml` is hand-parsed in `crates/sonus-core/src/config.rs` (line-by-line key=value, no inline comments, no nested tables).
- LRCLib User-Agent must identify as `sonus/<version>`; currently hardcoded in `crates/sonus-core/src/api/client.rs`.
- Do not change `yt-dlp` extractor args `--extractor-args "youtubemusicapp:player_client=ios_music"` in `crates/sonus-core/src/player/stream.rs`; this selects the correct YouTube Music stream.

## Change safety
- Do not block the event loop in `crates/sonus-tui/src/app.rs`. It uses `crossterm::event::poll(100–250ms)` + `try_recv()` — both non-blocking. Any blocking call (sync HTTP, `recv()`, blocking I/O) freezes the entire UI. Offload work via `tokio::spawn` and send results through a `tokio::sync::oneshot` or the existing `mpsc` channel.
- The player runs on a **`std::thread`**, not tokio. Commands to the player use `std::sync::mpsc`; events from the player use `tokio::sync::mpsc::unbounded`. Do not swap these channel types.
- Only one async API call (`pending_api`) can be in-flight at a time — search, lyrics fetch, recommendation load, and playlist load are mutually exclusive. New API features must check `self.pending_api.is_none()` before spawning.
- Preserve backward compatibility for the SQLite schema (tables: `playlists`, `playlist_tracks`, `history`).
- Do not alter connection-open PRAGMAs (`journal_mode=WAL`, `foreign_keys=ON`).
- Maintain the quadratic volume curve `sink.set_volume(v * v)` in `crates/sonus-core/src/player/volume.rs` — this is intentional perceptual scaling, not a bug.
- Maintain dual concurrent `yt-dlp` processes in `crates/sonus-core/src/player/stream.rs`: one pipes stdout to the decoder for real-time playback, one writes to cache. Making them sequential doubles playback latency.
- Maintain cache LRU pruning by mtime; `set_modified(SystemTime::now())` on cache hit is intentional, not a bug.

## Known gotchas
- Audio output init (`OutputStreamBuilder::open_default_stream`) fails silently via `Message::Error` instead of panicking.
- Database and cache directories are created lazily; failure to open the DB panics.
- No tests exist in either crate. There is no CI.
- Config parser silently ignores unknown keys and does not support TOML features beyond simple `key = value` / `key = "value"` lines.
