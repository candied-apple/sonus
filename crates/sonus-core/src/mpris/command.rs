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
