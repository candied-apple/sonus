use super::http::shared_http_client;

pub async fn get_lyric_from_lrclib(
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
