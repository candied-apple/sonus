use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rodio::{OutputStreamBuilder, Sink};

pub mod cache;
pub mod stream;
pub mod volume;

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

pub fn spawn(
    cmd_rx: mpsc::Receiver<PlayerCommand>,
    evt_tx: tokio::sync::mpsc::UnboundedSender<PlayerEvent>,
) {
    cache::warm_cache_estimate();
    thread::spawn(move || {
        let stream = match OutputStreamBuilder::open_default_stream() {
            Ok(s) => s,
            Err(e) => {
                let _ = evt_tx.send(PlayerEvent::Error(format!("Audio output: {}", e)));
                return;
            }
        };

        let sink = Sink::connect_new(stream.mixer());
        let mut state = stream::PlayerState {
            sink: Some(Arc::new(sink)),
            _stream: Some(stream),
            current_video_id: None,
            current_title: None,
            current_artist: None,
            duration_secs: 0.0,
            volume: crate::config::default_volume(),
            stream: None,
            loading_cancelled: None,
            start_secs: 0.0,
        };

        let (stream_tx, stream_rx) = mpsc::channel::<(String, stream::StreamState)>();
        let progress_interval = Duration::from_millis(250);
        let mut last_progress = std::time::Instant::now();

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(250)) {
                Ok(cmd) => {
                    match cmd {
                        PlayerCommand::Play {
                            video_id,
                            title,
                            artist,
                            duration_secs,
                        } => {
                            stream::play_track(
                                &mut state,
                                &video_id,
                                &title,
                                &artist,
                                duration_secs,
                                0.0,
                                &evt_tx,
                                &stream_tx,
                            );
                            last_progress = std::time::Instant::now();
                        }
                        PlayerCommand::Pause => {
                            if let Some(sink) = &state.sink {
                                sink.pause();
                            }
                        }
                        PlayerCommand::Resume => {
                            if let Some(sink) = &state.sink {
                                sink.play();
                            }
                        }
                        PlayerCommand::Stop => {
                            state.stop_current_stream();
                            if let Some(sink) = &state.sink {
                                sink.stop();
                            }
                            state.current_video_id = None;
                            state.current_title = None;
                            state.current_artist = None;
                        }
                        PlayerCommand::SetVolume(v) => {
                            state.volume = v;
                            if let Some(sink) = &state.sink {
                                sink.set_volume(volume::apply_volume_curve(v));
                            }
                        }
                        PlayerCommand::Seek(secs) => {
                            if let (Some(video_id), Some(title), Some(artist)) = (
                                state.current_video_id.clone(),
                                state.current_title.clone(),
                                state.current_artist.clone(),
                            ) {
                                let duration = state.duration_secs;
                                stream::play_track(
                                    &mut state,
                                    &video_id,
                                    &title,
                                    &artist,
                                    duration,
                                    secs,
                                    &evt_tx,
                                    &stream_tx,
                                );
                                last_progress = std::time::Instant::now();
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    while let Ok((video_id, stream_state)) = stream_rx.try_recv() {
                        if Some(&video_id) == state.current_video_id.as_ref() {
                            state.stream = Some(stream_state);
                        } else {
                            let mut st = stream_state;
                            st.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
                            if let Some(mut child) = st.ytdlp_child.take() {
                                let _ = child.kill();
                                let _ = child.wait();
                            }
                            if let Some(handle) = st.decode_thread.take() {
                                let _ = handle.join();
                            }
                        }
                    }

                    if last_progress.elapsed() >= progress_interval {
                        if let Some(sink) = &state.sink {
                            if !sink.empty() && state.stream.is_some() {
                                let pos = state.start_secs + sink.get_pos().as_secs_f64();
                                let _ = evt_tx
                                    .send(PlayerEvent::Progress(pos.min(state.duration_secs), state.duration_secs));
                            } else if sink.empty() && state.stream.is_some() {
                                stream::cleanup_natural_end(&mut state);
                                let _ = evt_tx.send(PlayerEvent::Finished);
                                state.current_video_id = None;
                                state.current_title = None;
                                state.current_artist = None;
                            }
                        }
                        last_progress = std::time::Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}
