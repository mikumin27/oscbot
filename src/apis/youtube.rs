use google_youtube3 as youtube;
use tokio::time::{Instant, sleep};
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use youtube::api::{Video, VideoSnippet, VideoStatus};
use youtube::{hyper_rustls, hyper_util, yup_oauth2, YouTube};

use crate::Error;

pub async fn wait_open(path: &Path, timeout: Duration) -> std::io::Result<std::fs::File> {
    let end = Instant::now() + timeout;

    loop {
        match tokio::fs::File::open(path).await {
            Ok(f) => return Ok(f.into_std().await),
            Err(e) if Instant::now() < end => {
                sleep(Duration::from_millis(25)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

// thank god for chatGPT
pub async fn upload(video_path: &String, title: String, description: String, thumbnail: Vec<u8>) -> Result<String, Error> {

    // Always persist OAuth tokens into the project working directory.
    // This is intentionally not configurable to keep local + Docker behavior identical.
    let token_path = "token.json";

    // Read OAuth client secret downloaded from Google Cloud Console
    let secret = yup_oauth2::read_application_secret("youtube_secret.json").await?;

    // Installed-app OAuth flow (opens browser / local redirect)
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_path)
    .build()
    .await?;

    // Hyper client (as in google-youtube3 README)
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        );

    let hub = YouTube::new(client, auth);

    let mut video = Video::default();

    let mut snippet = VideoSnippet::default();
    snippet.title = Some(title);
    snippet.description = Some(description);
    snippet.tags = Some(vec!["osu".into(), "switzerland".into(), "osc".into(), "osu!swisscommunity".into(), "osuswiss".into(), ]);
    snippet.category_id = Some("20".into());

    let mut status = VideoStatus::default();
    status.privacy_status = Some("unlisted".to_string()); // "public" | "unlisted" | "private"
    status.self_declared_made_for_kids = Some(false);
    video.snippet = Some(snippet);
    video.status = Some(status);

    // --- Upload ---
    // Use upload_resumable for larger files / flaky networks.
    // MIME can be video/* or application/octet-stream.
    let file = wait_open(Path::new(&video_path), Duration::from_secs(10)).await?;
    let mime = "video/mp4".parse().unwrap();

    let (_resp, uploaded_video) = hub
        .videos()
        .insert(video)
        // minimal scope for uploads:
        .add_scope(youtube::api::Scope::Upload)
        // ensure these parts appear in the response:
        .add_part("snippet")
        .add_part("status")
        .upload_resumable(file, mime)
        .await?;

    println!("Uploaded! id={:?}", uploaded_video.id);
    let video_id = uploaded_video.id.unwrap();

    if thumbnail.is_empty() {
        println!("No thumbnail bytes provided; skipping thumbnail set.");
        return Ok(video_id);
    }

    let thumb_mime = "image/png".parse().unwrap(); // or "image/jpeg"

    // set thumbnail
    let (_resp, thumb_resp) = hub
        .thumbnails()
        .set(&video_id)
        .add_scope(youtube::api::Scope::Upload)
        .upload(Cursor::new(thumbnail), thumb_mime)
        .await?;

    println!("Thumbnail set! items={:?}", thumb_resp.items);

    Ok(video_id)
}