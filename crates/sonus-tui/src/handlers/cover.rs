use tokio::sync::oneshot;
use crate::app::App;

impl App {
    pub(crate) fn fetch_cover_image(&mut self, video_id: String) {
        if !sonus_core::util::is_valid_video_id(&video_id) {
            self.current_cover_video_id = Some(video_id);
            return;
        }
        if self.current_cover_video_id.as_ref() == Some(&video_id) {
            return;
        }
        self.current_cover_video_id = Some(video_id.clone());
        self.cover_image = None;

        let (tx, rx) = oneshot::channel();
        self.pending_cover = Some(rx);

        tokio::spawn(async move {
            let url = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", video_id);
            match sonus_core::api::shared_http_client().get(url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if bytes.len() > 5 * 1024 * 1024 {
                                let _ = tx.send(Err("Image too large (>5MB)".to_string()));
                                return;
                            }
                            let _ = tx.send(Ok(bytes.to_vec()));
                            return;
                        }
                    }
                    let _ = tx.send(Err("Failed to read image bytes".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        });
    }
}
