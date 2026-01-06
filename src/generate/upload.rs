use poise::serenity_prelude::CreateAttachment;
use rosu_v2::prelude as rosu;

use crate::{Error, apis::{nerinyan, youtube}, discord_helper::{ContextForFunctions}, embeds, generate::{danser, thumbnail, youtube_text}};

pub async fn render_and_upload_by_score(
    cff: &ContextForFunctions<'_>,
    score: rosu::Score,
    map: rosu::BeatmapExtended,
    subtitle: Option<String>
) -> Result<(), Error> {
    let title = youtube_text::generate_title_with_score(&score, &map).await;
    cff.edit(embeds::render_and_upload_embed(&title, false, None, false)?, vec![]).await?;
    let thumbnail = thumbnail::generate_thumbnail_from_score(&score, &map, &subtitle.unwrap_or("".to_string())).await;
    let description = youtube_text::generate_description(score.user_id, map.map_id, Some(&score), None);

    render_and_upload(cff, &score.id.to_string(), &score.user_id, &map.mapset_id, &map.checksum.unwrap(), title, description, thumbnail).await?;
    Ok(())
}

pub async fn render_and_upload_by_replay(
    cff: &ContextForFunctions<'_>,
    replay: osu_db::Replay,
    map: rosu::BeatmapExtended,
    user: rosu::UserExtended,
    subtitle: Option<String>
) -> Result<(), Error> {
    let title = youtube_text::generate_title_with_replay(&replay, &map).await;
    cff.edit(embeds::render_and_upload_embed(&title, false, None, false)?, vec![]).await?;
    let timestamp = replay.timestamp.format("%d.%m.%Y at %H:%M").to_string();
    let thumbnail = thumbnail::generate_thumbnail_from_replay_file(&replay, &map, &subtitle.unwrap_or("".to_string())).await;
    let description = youtube_text::generate_description(user.user_id, map.map_id, None, Some(timestamp));
    render_and_upload(cff, &replay.replay_hash.unwrap(), &user.user_id, &map.mapset_id, &map.checksum.unwrap(), title, description, thumbnail).await?;

    Ok(())
}

pub async fn render_and_upload(
    cff: &ContextForFunctions<'_>,
    replay_reference: &String,
    user_id: &u32,
    mapset_id: &u32,
    map_hash: &String,
    title: String,
    description: String,
    thumbnail: Vec<u8>
) -> Result<(), Error> {
    nerinyan::download_mapset(cff, mapset_id).await?;
    let replay_bytes = danser::get_replay_bytes(&replay_reference, &map_hash).await?;
    cff.edit(embeds::render_and_upload_embed(&title, true, None, false)?, vec![]).await?;
    let replay_path = danser::render(cff, &title, map_hash, replay_reference, user_id).await?;
    let title_too_long = title.len() > 100;
    let video_title = if title_too_long {&"temporary title please replace".to_string()} else {&title};
    let video_id = youtube::upload(&replay_path, video_title.clone(), description, thumbnail).await.unwrap();
    cff.edit(embeds::render_and_upload_embed(&title, true, Some("100%".to_string()), true)?, vec![]).await?;
    danser::cleanup_files(&map_hash, &replay_reference, &replay_path).await;
    cff.edit(embeds::upload_result_embed(&title, &video_id, title_too_long)?, vec![CreateAttachment::bytes(replay_bytes, "replay.osr")]).await?;
    Ok(())
}