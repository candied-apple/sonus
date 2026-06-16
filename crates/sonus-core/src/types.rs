
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TrackCategory {
    Song,
    Video,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackItem {
    pub index: usize,
    pub title: String,
    pub artist: String,
    pub duration: String,
    pub duration_secs: f64,
    pub is_playing: bool,
    pub video_id: Option<String>,
    pub album: Option<String>,
    pub category: TrackCategory,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PlayStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RepeatMode {
    None,
    One,
    All,
}

#[derive(Debug, Clone)]
pub struct SyncedLine {
    pub timestamp: f64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub status: PlayStatus,
    pub current_track: Option<(String, String)>,
    pub current_video_id: Option<String>,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            status: PlayStatus::Stopped,
            current_track: None,
            current_video_id: None,
            position: 0.0,
            duration: 0.0,
            volume: crate::config::default_volume(),
            shuffle: false,
            repeat: RepeatMode::None,
        }
    }
}
