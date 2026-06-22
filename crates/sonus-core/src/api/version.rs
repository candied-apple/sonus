use super::http::shared_http_client;

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
