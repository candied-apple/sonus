use ytmapi_rs::query::GetWatchPlaylistQuery;
use ytmapi_rs::YtMusic;
use ytmapi_rs::parse::SearchResultVideo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ytm = YtMusic::new_unauthenticated().await?;
    
    let test_queries = [
        "Lindsey Stirling A Plague Tale: Requiem (Official Cover Music Video)",
    ];
    
    for query_str in test_queries {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        // Search for videos
        let search_results = ytm.search_videos(query_str).await?;
        if search_results.is_empty() {
            println!("No results for: {}", query_str);
            continue;
        }
        
        let video_id = match &search_results[0] {
            ytmapi_rs::parse::SearchResultVideo::Video { video_id, .. } => video_id.clone(),
            _ => continue,
        };
        
        println!("\n--- Fetching watch playlist for VIDEO: {} ({:?}) ---", query_str, video_id);
        
        let query = GetWatchPlaylistQuery::new_from_video_id(video_id);
        let json = ytm.json_query(query).await?;
        let value = json.into_inner();
        
        let contents_path = "/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer\
                     /watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content\
                     /musicQueueRenderer/content/playlistPanelRenderer/contents";
        
        if let Some(contents) = value.pointer(contents_path).and_then(|v| v.as_array()) {
            for (i, item) in contents.iter().take(15).enumerate() {
                let renderer = item.pointer("/playlistPanelVideoRenderer")
                    .or_else(|| item.pointer("/playlistPanelVideoWrapperRenderer/primaryRenderer/playlistPanelVideoRenderer"));
                
                if let Some(renderer) = renderer {
                    let title = renderer.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or("");
                    let music_video_type = renderer
                        .pointer("/navigationEndpoint/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType")
                        .and_then(|v| v.as_str());
                    
                    let runs = renderer.pointer("/longBylineText/runs").and_then(|v| v.as_array());
                    let has_album_endpoint = runs.map(|r| {
                        r.iter().any(|run| {
                            run.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
                                .and_then(|v| v.as_str()) == Some("MUSIC_PAGE_TYPE_ALBUM")
                        })
                    }).unwrap_or(false);
                    
                    let category = match music_video_type {
                        Some("MUSIC_VIDEO_TYPE_ATV") | Some("MUSIC_VIDEO_TYPE_PRIVATELY_OWNED_TRACK") => "Song",
                        _ => {
                            if has_album_endpoint {
                                "Song"
                            } else {
                                "Video"
                            }
                        }
                    };
                    
                    let byline = runs.map(|r| r.iter().filter_map(|run| run.pointer("/text").and_then(|v| v.as_str())).collect::<Vec<_>>().join("")).unwrap_or_default();

                    println!("{}. {} - Category: {}, Type: {:?}, Byline: {}", i + 1, title, category, music_video_type, byline);
                }
            }
        }
    }
    
    Ok(())
}
