pub enum PlayerCommand {
    Play {
        video_id: String,
        title: String,
        artist: String,
        duration_secs: f64,
    },
    Pause,
    Resume,
    Stop,
    SetVolume(f64),
    Seek(f64),
}

pub enum PlayerEvent {
    NowPlaying(String, String),
    Progress(f64, f64),
    Finished,
    Error(String),
    StatusMessage(String),
}
