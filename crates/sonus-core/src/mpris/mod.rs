use std::sync::{mpsc, Arc, RwLock};
use std::time::Duration;
use souvlaki::{MediaControls, MediaControlEvent, MediaMetadata, PlatformConfig, MediaPlayback, MediaPosition, SeekDirection};

/// Thread-safe shared state between App (main thread) and MPRIS (D-Bus/OS thread).
/// Written by App event loop, read by MPRIS/OS interface methods.
pub struct MprisState {
    pub playback_status: String,
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub video_id: Option<String>,
    pub position_micros: i64,
    pub length_micros: i64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: String,
    pub can_go_next: bool,
    pub can_go_previous: bool,
}

impl MprisState {
    pub fn new() -> Self {
        Self {
            playback_status: "Stopped".into(),
            track_id: String::new(),
            title: String::new(),
            artist: String::new(),
            album: None,
            video_id: None,
            position_micros: 0,
            length_micros: 0,
            volume: crate::config::default_volume(),
            shuffle: false,
            repeat: "None".into(),
            can_go_next: false,
            can_go_previous: false,
        }
    }
}

/// Commands from the MPRIS thread to the main App event loop.
pub enum MprisCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    Seek(f64),
    OpenUri(String),
    SetVolume(f64),
    SetShuffle(bool),
    SetRepeat(String),
}

/// Signals from the App event loop to the MPRIS thread.
pub enum MprisSignal {
    PropertiesChanged,
    Seeked(i64),
    Shutdown,
}

#[cfg(target_os = "windows")]
extern "system" {
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
fn get_hwnd() -> Option<*mut std::ffi::c_void> {
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_hwnd() -> Option<*mut std::ffi::c_void> {
    None
}

// ── Spawn ──

pub fn spawn(
    mpris_cmd_tx: mpsc::Sender<MprisCommand>,
    state: Arc<RwLock<MprisState>>,
    signal_rx: tokio::sync::mpsc::UnboundedReceiver<MprisSignal>,
) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                crate::log!("sonus: failed to create media controls runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = serve(mpris_cmd_tx, state, signal_rx).await {
                crate::log!("sonus: media controls server exited: {}", e);
            }
        });
    });
}

async fn serve(
    mpris_cmd_tx: mpsc::Sender<MprisCommand>,
    state: Arc<RwLock<MprisState>>,
    mut signal_rx: tokio::sync::mpsc::UnboundedReceiver<MprisSignal>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = PlatformConfig {
        dbus_name: "org.mpris.MediaPlayer2.sonus",
        display_name: "sonus",
        hwnd: get_hwnd(),
    };

    let mut controls = MediaControls::new(config)?;

    let tx_clone = mpris_cmd_tx.clone();
    let state_clone = state.clone();
    controls.attach(move |event| {
        let cmd = match event {
            MediaControlEvent::Play => MprisCommand::Play,
            MediaControlEvent::Pause => MprisCommand::Pause,
            MediaControlEvent::Toggle => MprisCommand::PlayPause,
            MediaControlEvent::Stop => MprisCommand::Stop,
            MediaControlEvent::Next => MprisCommand::Next,
            MediaControlEvent::Previous => MprisCommand::Previous,
            MediaControlEvent::Seek(SeekDirection::Forward) => {
                let current = state_clone.read().unwrap().position_micros as f64 / 1_000_000.0;
                MprisCommand::Seek(current + 10.0)
            }
            MediaControlEvent::Seek(SeekDirection::Backward) => {
                let current = state_clone.read().unwrap().position_micros as f64 / 1_000_000.0;
                MprisCommand::Seek((current - 10.0).max(0.0))
            }
            MediaControlEvent::SeekBy(direction, duration) => {
                let current = state_clone.read().unwrap().position_micros as f64 / 1_000_000.0;
                let mut secs = duration.as_secs_f64();
                if direction == SeekDirection::Backward {
                    secs = -secs;
                }
                MprisCommand::Seek((current + secs).max(0.0))
            }
            MediaControlEvent::SetPosition(MediaPosition(duration)) => {
                MprisCommand::Seek(duration.as_secs_f64())
            }
            MediaControlEvent::SetVolume(vol) => MprisCommand::SetVolume(vol),
            _ => return,
        };
        let _ = tx_clone.send(cmd);
    })?;

    // On macOS, start CFRunLoop in background to handle system media events
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(|| {
            extern "C" {
                fn CFRunLoopRun();
            }
            unsafe {
                CFRunLoopRun();
            }
        });
    }

    while let Some(signal) = signal_rx.recv().await {
        match signal {
            MprisSignal::PropertiesChanged => {
                let state_guard = state.read().unwrap();
                
                // 1. Update Playback Status
                let playback = match state_guard.playback_status.as_str() {
                    "Playing" => MediaPlayback::Playing {
                        progress: Some(MediaPosition(Duration::from_micros(state_guard.position_micros as u64))),
                    },
                    "Paused" => MediaPlayback::Paused {
                        progress: Some(MediaPosition(Duration::from_micros(state_guard.position_micros as u64))),
                    },
                    _ => MediaPlayback::Stopped,
                };
                let _ = controls.set_playback(playback);

                // 2. Update Metadata
                let mut art_url = None;
                if let Some(ref vid) = state_guard.video_id {
                    art_url = Some(format!("https://img.youtube.com/vi/{}/hqdefault.jpg", vid));
                }

                let duration = if state_guard.length_micros > 0 {
                    Some(Duration::from_micros(state_guard.length_micros as u64))
                } else {
                    None
                };

                let metadata = MediaMetadata {
                    title: if state_guard.title.is_empty() { None } else { Some(&state_guard.title) },
                    artist: if state_guard.artist.is_empty() { None } else { Some(&state_guard.artist) },
                    album: state_guard.album.as_deref(),
                    duration,
                    cover_url: art_url.as_deref(),
                };
                let _ = controls.set_metadata(metadata);
            }
            MprisSignal::Seeked(pos_micros) => {
                // If seeked, update playback progress directly
                let state_guard = state.read().unwrap();
                let playback = match state_guard.playback_status.as_str() {
                    "Playing" => MediaPlayback::Playing {
                        progress: Some(MediaPosition(Duration::from_micros(pos_micros as u64))),
                    },
                    "Paused" => MediaPlayback::Paused {
                        progress: Some(MediaPosition(Duration::from_micros(pos_micros as u64))),
                    },
                    _ => MediaPlayback::Stopped,
                };
                let _ = controls.set_playback(playback);
            }
            MprisSignal::Shutdown => {
                break;
            }
        }
    }

    Ok(())
}
