use std::fs::remove_file;

use poise::CreateReply;
use poise::serenity_prelude::{self as serenity, CreateActionRow, CreateAttachment};
use rosu_v2::prelude as rosu;
use crate::apis::{youtube};
use crate::embeds::single_text_response;
use crate::{Context, Error, embeds};
use crate::osu;
use crate::discord_helper::{ContextForFunctions, MessageState};
use crate::generate::{danser, thumbnail, upload};

#[poise::command(slash_command, rename = "dev", subcommands("test_osu_client", "test_thumbnail", "test_danser_and_youtube", "regenerate_token", "test_upload"))]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command)]
pub async fn test_osu_client(ctx: Context<'_>) -> Result<(), Error> {
    let score = osu::get_osu_instance().score(1724681877).await.expect("Score should exist");
    let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap exists");
    let embed = embeds::score_embed_from_score(&score, &map, None).await?;
    let button_id = format!("thumbnail:{}", score.id);
    let button = serenity::CreateButton::new(button_id)
    .label("Render Thumbnail")
    .emoji(crate::emojis::SATA_ANDAGI)
    .style(serenity::ButtonStyle::Primary);

    ctx.send(
        poise::CreateReply::default()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(vec![button])])
    ).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn test_thumbnail(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let score = osu::get_osu_instance().score(1611084369).await.expect("Score should exist");
    let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap exists");
    let image = thumbnail::generate_thumbnail_from_score(&score, &map, "Cool subtitle that i definitely just added").await;
    ctx.send(poise::CreateReply::default().attachment(CreateAttachment::bytes(image, "thumbnail.png"))).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn test_danser_and_youtube (
    ctx: Context<'_>,
    scorefile: serenity::Attachment
) -> Result<(), Error> {
    let reply = ctx.send(CreateReply::default().embed(embeds::render_and_upload_embed(&"...".into(), false, None, false)?)).await?;

    let bytes = scorefile.download().await?;
    let replay = match osu_db::Replay::from_bytes(&bytes) {
        Ok(replay) => replay,
        Err(_) => {
            embeds::single_text_response(&ctx, "Replay could not be parsed", MessageState::ERROR, false).await;
            return Ok(());
        },
    };

    let replay_hash = replay.replay_hash.as_ref().unwrap();
    let map: rosu::BeatmapExtended = match osu::get_beatmap_from_checksum(&replay.beatmap_hash).await {
        Some(map) => map,
        None => {
            embeds::single_text_response(&ctx, "Cannot find map related to the replay", MessageState::WARN, false).await;
            return Ok(());
        },
    };
    let user = match osu::get_osu_instance().user(replay.player_name.as_ref().expect("User must exist")).await {
        Ok(user) => user,
        _ => {
            single_text_response(&ctx, &format!("Cannot find user from replay"), MessageState::ERROR, false).await;
            return Ok(())
        }
    };
    let beatmap_hash = replay.beatmap_hash.as_ref().unwrap();
    danser::attach_replay(&beatmap_hash, replay_hash, &bytes).await?;
    let cff = ContextForFunctions {
        command_context: Some(ctx),
        reply: Some(reply),
        event_context: None,
        component: None
    };

    upload::render_and_upload_by_replay(&cff, replay, map, user, None).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn test_upload(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    youtube::upload(&"videoForRegen/random.mp4".into(), "test".into(), "test".into(), vec![]).await?;
    single_text_response(&ctx, "regenerated token!", MessageState::SUCCESS, true).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn regenerate_token(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let token_path = std::env::var("OSC_BOT_YOUTUBE_TOKEN_PATH").unwrap_or_else(|_| "token.json".to_string());
    remove_file(token_path).ok();
    youtube::upload(&"videoForRegen/random.mp4".into(), "test".into(), "test".into(), vec![]).await?;
    single_text_response(&ctx, "regenerated token!", MessageState::SUCCESS, true).await;
    Ok(())
}
