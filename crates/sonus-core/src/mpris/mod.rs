use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};

use zbus::interface;
use zbus::Connection;

/// Thread-safe shared state between App (main thread) and MPRIS (D-Bus thread).
/// Written by App event loop, read by MPRIS interface methods.
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
#[allow(dead_code)]
pub enum MprisSignal {
    PropertiesChanged,
    Seeked(i64),
    Shutdown,
}

// ── Root interface: org.mpris.MediaPlayer2 ──

struct MprisRoot;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property)]
    fn identity(&self) -> &str {
        "sonus"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "sonus"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["http", "https"]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![
            "audio/webm",
            "audio/opus",
            "audio/mp4",
            "audio/mpeg",
            "audio/flac",
            "audio/ogg",
        ]
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_set_fullscreen(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn fullscreen(&self) -> bool {
        false
    }
}

// ── Player interface: org.mpris.MediaPlayer2.Player ──

struct MprisPlayer {
    state: Arc<RwLock<MprisState>>,
    mpris_cmd_tx: mpsc::Sender<MprisCommand>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // ── Properties ──

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.state.read().unwrap().playback_status.clone()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        self.state.read().unwrap().repeat.clone()
    }

    #[zbus(property)]
    fn set_loop_status(&mut self, value: String) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::SetRepeat(value));
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.state.read().unwrap().shuffle
    }

    #[zbus(property)]
    fn set_shuffle(&mut self, value: bool) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::SetShuffle(value));
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.state.read().unwrap().volume
    }

    #[zbus(property)]
    fn set_volume(&mut self, value: f64) {
        let _ = self
            .mpris_cmd_tx
            .send(MprisCommand::SetVolume(value.clamp(0.0, 1.0)));
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.state.read().unwrap().position_micros
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, zbus::zvariant::Value<'_>> {
        let (track_id, length_micros, video_id, title, artist, album) = {
            let state = self.state.read().unwrap();
            (
                state.track_id.clone(),
                state.length_micros,
                state.video_id.clone(),
                state.title.clone(),
                state.artist.clone(),
                state.album.clone(),
            )
        };

        let mut map = HashMap::new();

        if !track_id.is_empty() {
            map.insert("mpris:trackid".into(), zbus::zvariant::Value::from(track_id));
        }

        if length_micros > 0 {
            map.insert(
                "mpris:length".into(),
                zbus::zvariant::Value::from(length_micros),
            );
        }

        if let Some(ref id) = video_id {
            let art_url = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id);
            map.insert("mpris:artUrl".into(), zbus::zvariant::Value::from(art_url));

            let yt_url = format!("https://music.youtube.com/watch?v={}", id);
            map.insert("xesam:url".into(), zbus::zvariant::Value::from(yt_url));
        }

        if !title.is_empty() {
            map.insert("xesam:title".into(), zbus::zvariant::Value::from(title));
        }

        if !artist.is_empty() {
            map.insert("xesam:artist".into(), zbus::zvariant::Value::from(vec![artist]));
        }

        if let Some(ref a) = album {
            map.insert("xesam:album".into(), zbus::zvariant::Value::from(a.clone()));
        }

        map
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.state.read().unwrap().can_go_next
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.state.read().unwrap().can_go_previous
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }

    // ── Methods ──

    fn play(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::Play);
    }

    fn pause(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::Pause);
    }

    fn play_pause(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::PlayPause);
    }

    fn stop(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::Stop);
    }

    fn next(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::Next);
    }

    fn previous(&mut self) {
        let _ = self.mpris_cmd_tx.send(MprisCommand::Previous);
    }

    fn seek(&mut self, offset: i64) {
        let current_pos = self.state.read().unwrap().position_micros;
        let new_pos = (current_pos + offset).max(0);
        let _ = self
            .mpris_cmd_tx
            .send(MprisCommand::Seek(new_pos as f64 / 1_000_000.0));
    }

    fn set_position(&mut self, track_id: zbus::zvariant::ObjectPath<'_>, position: i64) {
        let current_track_id = self.state.read().unwrap().track_id.clone();
        if track_id.as_str() == current_track_id {
            let secs = position as f64 / 1_000_000.0;
            let _ = self.mpris_cmd_tx.send(MprisCommand::Seek(secs));
        }
    }

    fn open_uri(&mut self, uri: String) {
        if let Some(video_id) = extract_video_id(&uri) {
            let _ = self.mpris_cmd_tx.send(MprisCommand::OpenUri(video_id));
        }
    }
}

// ── Helper: extract YouTube video ID ──

fn extract_video_id(uri: &str) -> Option<String> {
    let url = uri.trim();

    if let Some(pos) = url.find("v=") {
        let after_v = &url[pos + 2..];
        let id = after_v.split('&').next().unwrap_or("");
        if !id.is_empty() && id.len() <= 20 {
            return Some(id.to_string());
        }
    }

    if url.contains("youtu.be/") {
        if let Some(pos) = url.find("youtu.be/") {
            let id = &url[pos + 9..];
            let id = id.split('?').next().unwrap_or("").split('/').next().unwrap_or("");
            if !id.is_empty() && id.len() <= 20 {
                return Some(id.to_string());
            }
        }
    }

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
                crate::log!("sonus: failed to create MPRIS runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = serve(mpris_cmd_tx, state, signal_rx).await {
                crate::log!("sonus: MPRIS server exited: {}", e);
            }
        });
    });
}

async fn serve(
    mpris_cmd_tx: mpsc::Sender<MprisCommand>,
    state: Arc<RwLock<MprisState>>,
    mut signal_rx: tokio::sync::mpsc::UnboundedReceiver<MprisSignal>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::session().await?;
    conn.request_name("org.mpris.MediaPlayer2.sonus").await?;

    let root = MprisRoot;
    let player = MprisPlayer {
        state: state.clone(),
        mpris_cmd_tx,
    };

    conn.object_server()
        .at("/org/mpris/MediaPlayer2", root)
        .await?;
    conn.object_server()
        .at("/org/mpris/MediaPlayer2", player)
        .await?;

    while let Some(signal) = signal_rx.recv().await {
        match signal {
            MprisSignal::PropertiesChanged => {
                if let Err(e) = emit_properties_changed(&conn).await {
                    crate::log!("sonus: failed to emit PropertiesChanged: {}", e);
                }
            }
            MprisSignal::Seeked(pos) => {
                if let Err(e) = conn
                    .emit_signal(
                        None::<&str>,
                        "/org/mpris/MediaPlayer2",
                        "org.mpris.MediaPlayer2.Player",
                        "Seeked",
                        &(pos,),
                    )
                    .await
                {
                    crate::log!("sonus: failed to emit Seeked: {}", e);
                }
            }
            MprisSignal::Shutdown => return Ok(()),
        }
    }

    Ok(())
}

async fn emit_properties_changed(conn: &Connection) -> zbus::Result<()> {
    conn.emit_signal(
        None::<&str>,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "PropertiesChanged",
        &(
            "org.mpris.MediaPlayer2.Player",
            HashMap::<&str, zbus::zvariant::Value>::new(),
            vec![
                "PlaybackStatus",
                "Metadata",
                "Volume",
                "Position",
                "Shuffle",
                "LoopStatus",
            ],
        ),
    )
    .await
}
