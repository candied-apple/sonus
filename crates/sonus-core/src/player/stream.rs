use std::io::BufReader;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use rodio::Sink;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::default::register_enabled_codecs;
use symphonia_adapter_libopus::OpusDecoder;
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;

use super::cache;
use super::volume;
use super::PlayerEvent;

// ---------------------------------------------------------------------------
// StreamSource — a rodio Source backed by an mpsc channel of sample chunks
// ---------------------------------------------------------------------------

struct StreamSource {
    rx: mpsc::Receiver<Vec<f32>>,
    current_chunk: Vec<f32>,
    current_idx: usize,
    sample_rate: u32,
    channels: u16,
    cancelled: Arc<AtomicBool>,
}

impl Iterator for StreamSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.cancelled.load(Ordering::SeqCst) {
            return None;
        }
        if self.current_idx < self.current_chunk.len() {
            let s = self.current_chunk[self.current_idx];
            self.current_idx += 1;
            return Some(s);
        }
        match self.rx.recv() {
            Ok(chunk) => {
                self.current_chunk = chunk;
                self.current_idx = 0;
                self.next()
            }
            Err(_) => None,
        }
    }
}

impl rodio::Source for StreamSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ---------------------------------------------------------------------------
// Internal state for the currently-playing stream
// ---------------------------------------------------------------------------

pub(super) struct StreamState {
    pub(super) ytdlp_child: Option<Child>,
    pub(super) decode_thread: Option<thread::JoinHandle<()>>,
    pub(super) cancelled: Arc<AtomicBool>,
    pub(super) download_child: Option<Arc<std::sync::Mutex<Option<Child>>>>,
}

// ---------------------------------------------------------------------------
// Player state
// ---------------------------------------------------------------------------

pub(super) struct PlayerState {
    pub(super) sink: Option<Arc<Sink>>,
    pub(super) _stream: Option<rodio::OutputStream>,
    pub(super) current_video_id: Option<String>,
    pub(super) current_title: Option<String>,
    pub(super) current_artist: Option<String>,
    pub(super) duration_secs: f64,
    pub(super) volume: f64,
    pub(super) stream: Option<StreamState>,
    pub(super) loading_cancelled: Option<Arc<AtomicBool>>,
    pub(super) start_secs: f64,
}

impl PlayerState {
    pub(super) fn stop_current_stream(&mut self) {
        if let Some(cancel_flag) = self.loading_cancelled.take() {
            cancel_flag.store(true, Ordering::SeqCst);
        }
        if let Some(mut st) = self.stream.take() {
            st.cancelled.store(true, Ordering::SeqCst);
            if let Some(mut child) = st.ytdlp_child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            if let Some(download_mutex) = st.download_child.take() {
                if let Ok(mut guard) = download_mutex.lock() {
                    if let Some(mut child) = guard.take() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
            if let Some(handle) = st.decode_thread.take() {
                let _ = handle.join();
            }
        }
    }
}

pub(super) fn cleanup_natural_end(state: &mut PlayerState) {
    if let Some(mut st) = state.stream.take() {
        st.cancelled.store(true, Ordering::SeqCst);
        if let Some(mut child) = st.ytdlp_child.take() {
            let _ = child.wait();
        }
        if let Some(handle) = st.decode_thread.take() {
            let _ = handle.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn play_track(
    state: &mut PlayerState,
    video_id: &str,
    title: &str,
    artist: &str,
    duration_secs: f64,
    start_secs: f64,
    evt_tx: &tokio::sync::mpsc::UnboundedSender<PlayerEvent>,
    stream_tx: &std::sync::mpsc::Sender<(String, StreamState)>,
) {
    if !crate::util::is_valid_video_id(video_id) {
        let _ = evt_tx.send(PlayerEvent::Error(format!("Invalid video_id: {}", video_id)));
        return;
    }

    state.stop_current_stream();

    if let Some(sink) = &state.sink {
        sink.stop();
    }

    state.duration_secs = duration_secs;
    state.start_secs = start_secs;
    state.current_video_id = Some(video_id.to_string());
    state.current_title = Some(title.to_string());
    state.current_artist = Some(artist.to_string());

    let cancelled = Arc::new(AtomicBool::new(false));
    state.loading_cancelled = Some(cancelled.clone());

    let cache_path = cache::get_cache_dir().join(format!("{}.webm", video_id));
    let is_cached = cache_path.is_file();

    let tmp_path = cache::get_cache_dir().join(format!("{}.tmp", video_id));
    let _ = std::fs::remove_file(&tmp_path);

    let url = format!("https://music.youtube.com/watch?v={}", video_id);
    let download_child = if !is_cached {
        Command::new("yt-dlp")
            .args(["-f", "bestaudio[ext=webm]/bestaudio", "-o", tmp_path.to_str().unwrap(), &url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    } else {
        None
    };

    let shared_child = download_child.map(|c| Arc::new(std::sync::Mutex::new(Some(c))));

    if let Some(child_ref) = shared_child.clone() {
        let cancel_flag = cancelled.clone();
        let tmp_path_clone = tmp_path.clone();
        let cache_path_clone = cache_path.clone();
        thread::spawn(move || {
            let mut child_to_wait = None;
            if let Ok(mut guard) = child_ref.lock() {
                child_to_wait = guard.take();
            }
            if let Some(mut child) = child_to_wait {
                if let Ok(status) = child.wait() {
                    if status.success() && !cancel_flag.load(Ordering::SeqCst) {
                        if let Ok(()) = std::fs::rename(&tmp_path_clone, &cache_path_clone) {
                            if let Ok(meta) = std::fs::metadata(&cache_path_clone) {
                                cache::add_file_size(meta.len());
                            }
                            let _ = cache::prune_cache(crate::config::cache_limit_bytes());
                        }
                    } else {
                        let _ = std::fs::remove_file(&tmp_path_clone);
                    }
                }
            }
        });
    }

    let video_id_clone = video_id.to_string();
    let title_clone = title.to_string();
    let artist_clone = artist.to_string();
    let sink_clone = state.sink.clone();
    let evt_tx_clone = evt_tx.clone();
    let stream_tx_clone = stream_tx.clone();
    let volume_val = state.volume;
    let cache_path_clone = cache_path.clone();

    thread::spawn(move || {
        let url = format!("https://music.youtube.com/watch?v={}", video_id_clone);
        if cancelled.load(Ordering::SeqCst) {
            return;
        }

        let (mut child, mss): (Option<Child>, MediaSourceStream) = if is_cached {
            match std::fs::File::open(&cache_path_clone) {
                Ok(f) => {
                    if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&cache_path_clone) {
                        let _ = file.set_modified(std::time::SystemTime::now());
                    }
                    let mss = MediaSourceStream::new(Box::new(f), Default::default());
                    (None, mss)
                }
                Err(e) => {
                    let _ = evt_tx_clone.send(PlayerEvent::Error(format!("Cache file open: {}", e)));
                    return;
                }
            }
        } else {
            let mut cmd = Command::new("yt-dlp");
            cmd.args(["-f", "bestaudio[ext=webm]/bestaudio", "-o", "-"]);
            if start_secs > 0.0 {
                let section = format!("*{}-inf", start_secs);
                cmd.args(["--download-sections", &section]);
            }
            cmd.arg(&url);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::null());

            let mut c = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    if !cancelled.load(Ordering::SeqCst) {
                        let _ = evt_tx_clone.send(PlayerEvent::Error(format!("yt-dlp: {}", e)));
                    }
                    return;
                }
            };

            let stdout = match c.stdout.take() {
                Some(r) => r,
                None => {
                    let _ = c.kill();
                    let _ = c.wait();
                    if !cancelled.load(Ordering::SeqCst) {
                        let _ = evt_tx_clone.send(PlayerEvent::Error("no stdout from yt-dlp".into()));
                    }
                    return;
                }
            };
            let reader = BufReader::new(stdout);
            let source = ReadOnlySource::new(reader);
            let mss = MediaSourceStream::new(Box::new(source), Default::default());
            (Some(c), mss)
        };

        let hint = Default::default();
        let fmt_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();

        let mut format = match symphonia::default::get_probe()
            .probe(&hint, mss, fmt_opts, meta_opts)
        {
            Ok(f) => f,
            Err(e) => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = evt_tx_clone.send(PlayerEvent::Error(format!("Probe: {}", e)));
                }
                return;
            }
        };

        if cancelled.load(Ordering::SeqCst) {
            if let Some(mut c) = child.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
            return;
        }

        let track = match format.default_track(TrackType::Audio) {
            Some(t) => t.clone(),
            None => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = evt_tx_clone.send(PlayerEvent::Error("no audio track".into()));
                }
                return;
            }
        };

        let codec_params = match track.codec_params.as_ref() {
            Some(p) => p,
            None => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = evt_tx_clone.send(PlayerEvent::Error("no codec params".into()));
                }
                return;
            }
        };

        let audio_params = match codec_params.audio() {
            Some(a) => a,
            None => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = evt_tx_clone.send(PlayerEvent::Error("no audio codec params".into()));
                }
                return;
            }
        };

        let track_id = track.id;
        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let channels = audio_params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(2);

        static CODEC_REGISTRY: std::sync::OnceLock<CodecRegistry> = std::sync::OnceLock::new();
        let codec_registry = CODEC_REGISTRY.get_or_init(|| {
            let mut r = CodecRegistry::new();
            register_enabled_codecs(&mut r);
            r.register_audio_decoder::<OpusDecoder>();
            r
        });

        let dec_opts: AudioDecoderOptions = Default::default();
        let mut decoder = match codec_registry.make_audio_decoder(audio_params, &dec_opts) {
            Ok(d) => d,
            Err(e) => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                if !cancelled.load(Ordering::SeqCst) {
                    let _ = evt_tx_clone.send(PlayerEvent::Error(format!("Decoder: {}", e)));
                }
                return;
            }
        };

        if cancelled.load(Ordering::SeqCst) {
            if let Some(mut c) = child.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
            return;
        }

        if is_cached && start_secs > 0.0 {
            if let Some(time) = symphonia::core::units::Time::try_from_secs_f64(start_secs) {
                let seek_to = symphonia::core::formats::SeekTo::Time {
                    time,
                    track_id: Some(track_id),
                };
                let _ = format.seek(symphonia::core::formats::SeekMode::Coarse, seek_to);
            }
        }

        let (sample_tx, sample_rx) = mpsc::sync_channel::<Vec<f32>>(32);
        let cancelled_decode = cancelled.clone();

        let decode_thread = thread::spawn(move || {
            loop {
                if cancelled_decode.load(Ordering::SeqCst) {
                    break;
                }

                let packet = match format.next_packet() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => break,
                    Err(symphonia::core::errors::Error::ResetRequired) => {
                        decoder.reset();
                        continue;
                    }
                    Err(symphonia::core::errors::Error::IoError(ref e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        break;
                    }
                    Err(_) => break,
                };

                if packet.track_id != track_id {
                    continue;
                }

                if let Ok(decoded) = decoder.decode(&packet) {
                    let sample_count = decoded.samples_interleaved();
                    if sample_count == 0 {
                        continue;
                    }
                    let mut samples = vec![0.0f32; sample_count];
                    decoded.copy_to_slice_interleaved(&mut samples);

                    let mut samples = samples;
                    loop {
                        if cancelled_decode.load(Ordering::SeqCst) {
                            return;
                        }
                        match sample_tx.try_send(samples) {
                            Ok(()) => break,
                            Err(mpsc::TrySendError::Full(val)) => {
                                samples = val;
                                thread::sleep(Duration::from_millis(10));
                            }
                            Err(mpsc::TrySendError::Disconnected(_)) => return,
                        }
                    }
                }
            }
        });

        let source = StreamSource {
            rx: sample_rx,
            current_chunk: Vec::new(),
            current_idx: 0,
            sample_rate,
            channels,
            cancelled: cancelled.clone(),
        };

        if let Some(sink) = &sink_clone {
            sink.set_volume(volume::apply_volume_curve(volume_val));
            sink.append(source);
            sink.play();
        }

        let stream_state = StreamState {
            ytdlp_child: child,
            decode_thread: Some(decode_thread),
            cancelled: cancelled.clone(),
            download_child: shared_child,
        };

        let _ = stream_tx_clone.send((video_id_clone, stream_state));
        let _ = evt_tx_clone.send(PlayerEvent::NowPlaying(title_clone, artist_clone));
    });
}
