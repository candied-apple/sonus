use std::sync::Arc;
use std::time::Duration;

use ytmapi_rs::auth::noauth::NoAuthToken;
use ytmapi_rs::common::{PlaylistID, VideoID, YoutubeID};
use ytmapi_rs::parse::{ParsedSongArtist, PlaylistItem, SearchResultVideo};
use ytmapi_rs::query::GetWatchPlaylistQuery;
use ytmapi_rs::YtMusic;

use crate::types::{TrackCategory, TrackItem};
use crate::util;

pub fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client")
    })
}

pub struct YtmClient {
    inner: Option<Arc<YtMusic<NoAuthToken>>>,
}

fn artists_to_string(artists: &[ParsedSongArtist]) -> String {
    artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

impl YtmClient {
    pub async fn new() -> Self {
        match YtMusic::new_unauthenticated().await {
            Ok(client) => Self {
                inner: Some(Arc::new(client)),
            },
            Err(e) => {
                crate::log!("YTM init failed: {e}");
                Self { inner: None }
            }
        }
    }

    fn inner(&self) -> Result<&Arc<YtMusic<NoAuthToken>>, String> {
        self.inner
            .as_ref()
            .ok_or_else(|| "YTM client not initialized".into())
    }

    pub async fn search_songs(&self, query: &str) -> Result<Vec<TrackItem>, String> {
        let client = self.inner()?;
        let results = client
            .search_songs(query)
            .await
            .map_err(|e| format!("Search failed: {e}"))?;

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let dur = util::parse_time_string(&s.duration).unwrap_or(0.0);
                TrackItem {
                    index: i + 1,
                    title: s.title,
                    artist: s.artist,
                    duration: s.duration,
                    duration_secs: dur,
                    is_playing: false,
                    video_id: Some(s.video_id.get_raw().to_string()),
                    album: s.album.map(|a| a.name),
                    category: TrackCategory::Song,
                }
            })
            .collect())
    }

    pub async fn search_all(&self, query: &str) -> Result<Vec<TrackItem>, String> {
        let client = self.inner()?;

        let (songs_res, videos_res) = tokio::join!(
            client.search_songs(query),
            client.search_videos(query),
        );

        let mut tracks: Vec<TrackItem> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        match songs_res {
            Ok(songs) => {
                for s in songs {
                    let dur = util::parse_time_string(&s.duration).unwrap_or(0.0);
                    tracks.push(TrackItem {
                        index: 0,
                        title: s.title,
                        artist: s.artist,
                        duration: s.duration,
                        duration_secs: dur,
                        is_playing: false,
                        video_id: Some(s.video_id.get_raw().to_string()),
                        album: s.album.map(|a| a.name),
                        category: TrackCategory::Song,
                    });
                }
            }
            Err(e) => {
                errors.push(format!("Song search failed: {e}"));
            }
        }

        match videos_res {
            Ok(videos) => {
                for v in videos {
                    match v {
                        SearchResultVideo::Video {
                            title,
                            channel_name,
                            video_id,
                            length,
                            ..
                        } => {
                            let dur = util::parse_time_string(&length).unwrap_or(0.0);
                            tracks.push(TrackItem {
                                index: 0,
                                title,
                                artist: channel_name,
                                duration: length,
                                duration_secs: dur,
                                is_playing: false,
                                video_id: Some(video_id.get_raw().to_string()),
                                album: None,
                                category: TrackCategory::Video,
                            });
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Video search failed: {e}"));
            }
        }

        if tracks.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }

        if !errors.is_empty() {
            crate::log!("Partial search failure: {}", errors.join("; "));
        }

        for (i, t) in tracks.iter_mut().enumerate() {
            t.index = i + 1;
        }

        Ok(tracks)
    }

    pub async fn get_recommendations(&self, video_id: &str) -> Result<Vec<TrackItem>, String> {
        self.get_recommendations_categorized(video_id).await
    }

    async fn get_recommendations_fallback(&self, video_id: &str) -> Result<Vec<TrackItem>, String> {
        let client = self.inner()?;
        let tracks = client
            .get_watch_playlist_from_video_id(VideoID::from_raw(video_id))
            .await
            .map_err(|e| format!("Watch playlist fallback failed: {e}"))?;
        
        Ok(tracks.into_iter().filter_map(|t| {
            let dur = util::parse_time_string(&t.duration).unwrap_or(0.0);
            if dur <= 0.0 { return None; }
            
            // Heuristic for fallback: Topic channels are usually songs
            let is_topic = t.author.to_lowercase().contains(" - topic");
            
            let category = if is_topic { 
                TrackCategory::Song 
            } else if dur > 600.0 { 
                TrackCategory::Video 
            } else {
                // Without structural info, we guess based on duration or just default to Song
                // for the fallback. Most things in a watch playlist are relevant music.
                TrackCategory::Song
            };

            Some(TrackItem {
                index: 0,
                title: t.title,
                artist: t.author,
                duration: t.duration,
                duration_secs: dur,
                is_playing: false,
                video_id: Some(t.video_id.get_raw().to_string()),
                album: None,
                category,
            })
        }).collect())
    }

    pub async fn get_recommendations_categorized(&self, video_id: &str) -> Result<Vec<TrackItem>, String> {
        let client = self.inner()?;
        let query = GetWatchPlaylistQuery::new_from_video_id(VideoID::from_raw(video_id));
        
        let json_res = client.json_query(query).await;
        
        let value = match json_res {
            Ok(json) => json.into_inner(),
            Err(e) => {
                crate::log!("Watch playlist JSON query failed, trying fallback: {e}");
                return self.get_recommendations_fallback(video_id).await;
            }
        };

        let contents_path = "/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer\
                 /watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content\
                 /musicQueueRenderer/content/playlistPanelRenderer/contents";
        
        let contents = match value.pointer(contents_path).and_then(|v| v.as_array()) {
            Some(c) => c,
            None => {
                crate::log!("Failed to parse watch playlist response at path, trying fallback");
                return self.get_recommendations_fallback(video_id).await;
            }
        };


        let mut tracks = Vec::new();
        for item in contents {
            let renderer = item
                .pointer("/playlistPanelVideoRenderer")
                .or_else(|| {
                    item.pointer(
                        "/playlistPanelVideoWrapperRenderer/primaryRenderer\
                         /playlistPanelVideoRenderer",
                    )
                });

            let Some(renderer) = renderer else { continue };

            let title = renderer
                .pointer("/title/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let author = renderer
                .pointer("/shortBylineText/runs/0/text")
                .or_else(|| renderer.pointer("/longBylineText/runs/0/text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let duration = renderer
                .pointer("/lengthText/runs/0/text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let vid = renderer
                .pointer("/navigationEndpoint/watchEndpoint/videoId")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let dur = util::parse_time_string(duration).unwrap_or(0.0);
            if dur <= 0.0 || vid.is_empty() {
                continue;
            }

            let music_video_type = renderer
                .pointer(
                    "/navigationEndpoint/watchEndpoint\
                     /watchEndpointMusicSupportedConfigs\
                     /watchEndpointMusicConfig/musicVideoType",
                )
                .or_else(|| {
                     renderer.pointer("/playNavigationEndpoint/watchEndpoint\
                        /watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType")
                })
                .and_then(|v| v.as_str());

            let category = match music_video_type {
                | Some("MUSIC_VIDEO_TYPE_ATV")
                | Some("MUSIC_VIDEO_TYPE_PRIVATELY_OWNED_TRACK") => TrackCategory::Song,
                _ => {
                    let runs = renderer.pointer("/longBylineText/runs").and_then(|v| v.as_array());
                    let has_album_run = runs.map(|r| {
                        r.iter().any(|run| {
                            run.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
                                .and_then(|v| v.as_str()) == Some("MUSIC_PAGE_TYPE_ALBUM")
                        })
                    }).unwrap_or(false);

                    if has_album_run {
                        TrackCategory::Song
                    } else {
                        TrackCategory::Video
                    }
                }
            };

            let mut album = None;
            if let Some(runs) = renderer.pointer("/longBylineText/runs").and_then(|v| v.as_array()) {
                // Find the first run that has an album pageType, 
                // or the first run after the first "•" that doesn't look like views/date.
                let mut after_first_dot = false;
                for run in runs {
                    let text = run.pointer("/text").and_then(|v| v.as_str()).unwrap_or("").trim();
                    if text == "•" {
                        after_first_dot = true;
                        continue;
                    }
                    
                    if after_first_dot {
                        let is_album_type = run.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
                            .and_then(|v| v.as_str()) == Some("MUSIC_PAGE_TYPE_ALBUM");
                        
                        if is_album_type {
                            album = Some(text.to_string());
                            break;
                        }
                        
                        // Heuristic: if it doesn't look like metadata
                        if !text.contains("views") && !text.contains("likes") 
                            && !text.chars().all(|c| c.is_numeric() || c.is_whitespace() || c == ',') 
                            && !text.contains(":") && !text.is_empty() 
                        {
                            album = Some(text.to_string());
                            break;
                        }
                        
                        // If we found something that looks like views/year, we stop looking for album
                        if text.contains("views") || text.chars().all(|c| c.is_numeric() || c.is_whitespace() || c == ',') {
                            break;
                        }
                    }
                }
            }

            tracks.push(TrackItem {
                index: 0,
                title: title.to_string(),
                artist: author.to_string(),
                duration: duration.to_string(),
                duration_secs: dur,
                is_playing: false,
                video_id: Some(vid.to_string()),
                album,
                category,
            });
        }

        Ok(tracks)
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<TrackItem>, String> {
        let client = self.inner()?;
        let items = client
            .get_playlist_tracks(PlaylistID::from_raw(playlist_id))
            .await
            .map_err(|e| format!("Playlist tracks failed: {e}"))?;

        Ok(items
            .into_iter()
            .enumerate()
            .filter_map(|(i, item)| {
                match item {
                    PlaylistItem::Song(s) => {
                        let dur = util::parse_time_string(&s.duration).unwrap_or(0.0);
                        Some(TrackItem {
                            index: i + 1,
                            title: s.title,
                            artist: artists_to_string(&s.artists),
                            duration: s.duration,
                            duration_secs: dur,
                            is_playing: false,
                            video_id: Some(s.video_id.get_raw().to_string()),
                            album: Some(s.album.name),
                            category: TrackCategory::Song,
                        })
                    }
                    _ => None,
                }
            })
            .collect())
    }

    pub async fn get_lyrics(
        &self,
        video_id: &str,
        artist: &str,
        title: &str,
        duration_secs: f64,
    ) -> Result<(Option<String>, Option<String>), String> {
        if let Ok((plain, synced)) = self.get_lyric_from_lrclib(artist, title, duration_secs).await
        {
            if plain.is_some() || synced.is_some() {
                return Ok((plain, synced));
            }
        }

        let client = self.inner()?;
        if let Ok(lyrics_id) = client.get_lyrics_id(VideoID::from_raw(video_id)).await {
            if let Ok(lyrics) = client.get_lyrics(lyrics_id).await {
                return Ok((Some(lyrics.lyrics), None));
            }
        }

        Err("No lyrics available".to_string())
    }

    pub async fn get_ytm_lyrics(&self, video_id: &str) -> Result<String, String> {
        let client = self.inner()?;
        if let Ok(lyrics_id) = client.get_lyrics_id(VideoID::from_raw(video_id)).await {
            if let Ok(lyrics) = client.get_lyrics(lyrics_id).await {
                return Ok(lyrics.lyrics);
            }
        }
        Err("No YTM lyrics available".to_string())
    }

    pub async fn get_lyric_from_lrclib(
        &self,
        artist: &str,
        title: &str,
        duration_secs: f64,
    ) -> Result<(Option<String>, Option<String>), String> {
        let url = "https://lrclib.net/api/get";
        let res = shared_http_client()
            .get(url)
            .header("User-Agent", "sonus/0.2.0 ( https://github.com/alp/sonus )")
            .query(&[
                ("artist_name", artist),
                ("track_name", title),
                ("duration", &format!("{}", duration_secs.round() as u64)),
            ])
            .send()
            .await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    #[derive(serde::Deserialize)]
                    struct LrcResponse {
                        #[serde(rename = "plainLyrics")]
                        plain_lyrics: Option<String>,
                        #[serde(rename = "syncedLyrics")]
                        synced_lyrics: Option<String>,
                    }
                    if let Ok(data) = response.json::<LrcResponse>().await {
                        return Ok((data.plain_lyrics, data.synced_lyrics));
                    }
                }
                Err("No lyrics found in LRCLib".to_string())
            }
            Err(e) => Err(format!("LRCLib request failed: {e}")),
        }
    }
}

pub async fn check_latest_release() -> Result<String, String> {
    let url = "https://api.github.com/repos/candied-apple/sonus/releases/latest";
    let res = shared_http_client()
        .get(url)
        .header("User-Agent", "sonus/0.2.0")
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                #[derive(serde::Deserialize)]
                struct GithubRelease {
                    tag_name: String,
                }
                if let Ok(data) = response.json::<GithubRelease>().await {
                    return Ok(data.tag_name);
                }
            }
            Err("Failed to parse latest release".to_string())
        }
        Err(e) => Err(format!("GitHub Release request failed: {e}")),
    }
}

