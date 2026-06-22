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
