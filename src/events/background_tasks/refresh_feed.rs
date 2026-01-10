use poise::serenity_prelude::{self as serenity, CreateMessage};
use quick_xml::{events::Event, Reader};
use reqwest::StatusCode;
use std::{env, sync::{Mutex, OnceLock}};

use crate::{Error, defaults::NEW_VIDEOS_CHANNEL};

static LAST_VIDEO_ID: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();

fn value() -> &'static Mutex<Option<Vec<String>>> {
    LAST_VIDEO_ID.get_or_init(|| Mutex::new(None))
}

fn set(s: Vec<String>) {
    *value().lock().unwrap() = Some(s);
}

fn get_clone() -> Option<Vec<String>> {
    value().lock().unwrap().clone()
}

pub async fn refresh_feed(ctx: &serenity::Context) -> Result<(), Error> {
    tracing::debug!("checking youtube for new video uploads");
    let url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={}", env::var("OSC_BOT_YOUTUBE_CHANNEL_ID").unwrap());
    let last_video_ids = get_clone();
    tracing::debug!(last = last_video_ids.clone().unwrap_or(vec!["empty".to_string()]).join(", "));

    let c = reqwest::Client::new();
    let req = c.get(&url);

    let resp = req.send().await.unwrap();
    if resp.status() != StatusCode::OK { return Err(format!("http {}", resp.status()).into()); }

    let xml = resp.text().await?;
    let video_ids = get_video_ids(&xml)?;

    tracing::debug!(video_ids = video_ids.join(", "));


    let unwrapped_video_ids = match last_video_ids {
        Some(video_ids) => video_ids,
        None => {
            set(video_ids);
            tracing::debug!("first loop... checking for video uploads is skipped");
            return Ok(())
        }
    };

    for video_id in &video_ids {
        if unwrapped_video_ids.contains(video_id) {
            tracing::debug!("checking for new uploads has finished!");
            return Ok(())
        }
        set(video_ids.clone());
        tracing::info!(link = format!("https://youtu.be/{}", video_id), "New upload has been found!");
        NEW_VIDEOS_CHANNEL.send_message(ctx,
            CreateMessage::default().content(format!("A new score has been uploaded!\nhttps://youtu.be/{}", video_id))
        ).await?;
    }
    tracing::debug!("checking for new uploads has finished!");
    Ok(())
}

fn get_video_ids(xml: &str) -> Result<Vec<String>, Error> {
    let mut r = Reader::from_str(xml);
    r.config_mut().trim_text(true);
    let mut b = Vec::new();
    let mut in_id = false;
    let mut video_ids: Vec<String> = vec![];
    loop {
        match r.read_event_into(&mut b) {
            Ok(Event::Start(e)) if e.name().as_ref().ends_with(b"videoId") => in_id = true,
            Ok(Event::End(e))   if e.name().as_ref().ends_with(b"videoId") => in_id = false,
            Ok(Event::Text(t))  if in_id => video_ids.push(t.decode()?.into_owned()),
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        b.clear();
    }
    Ok(video_ids)
}
