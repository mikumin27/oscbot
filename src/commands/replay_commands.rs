use poise::{CreateReply, serenity_prelude as serenity};
use rosu_v2::prelude as rosu;
use rosu_v2::prelude::BeatmapExtended;
use crate::discord_helper::{ContextForFunctions, MessageState};
use crate::embeds::{single_text_response, single_text_response_embed};
use crate::{Context, Error, embeds};

use crate::osu;
use crate::generate::{danser, thumbnail, upload, youtube_text};
use crate::discord_helper::user_has_replay_role;

async fn has_replay_role(ctx: Context<'_>) -> Result<bool, Error> {
    if !user_has_replay_role(ctx, ctx.author()).await.unwrap() {
        single_text_response(&ctx, "No permission L", MessageState::INFO, true).await;
        return Ok(false);
    }
    Ok(true)
}

#[poise::command(slash_command, rename = "replay", subcommands("generate"), check = "has_replay_role")]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, subcommands("thumbnail", "title_and_description", "render_and_upload"), check = "has_replay_role")]
pub async fn generate(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

/// Either select score id or score file
#[poise::command(slash_command)]
pub async fn thumbnail(
    ctx: Context<'_>,
    #[description = "score id"] scoreid: Option<u64>,
    #[description = "score file"] scorefile: Option<serenity::Attachment>,
    #[description = "subtitle inside the thumbnail"] subtitle: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let image: Vec<u8>;

    if scoreid.is_some() {
        let unwrapped_score_id = scoreid.unwrap();
        let score = match osu::get_osu_instance().score(unwrapped_score_id).await {
            Ok(score) => score,
            Err(_) => {
                single_text_response(&ctx, &format!("Score with id {} does not exist", unwrapped_score_id), MessageState::WARN, false).await;
                return Ok(());
            }
        };
        let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap exists");
        image = thumbnail::generate_thumbnail_from_score(&score, &map, &subtitle.unwrap_or("".to_string())).await;
    }
    else if scorefile.is_some() {
        let bytes = scorefile.unwrap().download().await?;
        let replay = match osu_db::Replay::from_bytes(&bytes) {
            Ok(replay) => replay,
            Err(_) => {
                single_text_response(&ctx, "Replay could not be parsed", MessageState::ERROR, false).await;
                return Ok(());
            },
        };
        let map: BeatmapExtended = match osu::get_beatmap_from_checksum(&replay.beatmap_hash).await {
            Some(map) => map,
            None => {
                single_text_response(&ctx, "Cannot find map related to the replay", MessageState::WARN, false).await;
                return Ok(());
            },
        };
        image = thumbnail::generate_thumbnail_from_replay_file(&replay, &map, &subtitle.unwrap_or("".to_string())).await;
    }
    else {
        embeds::single_text_response(&ctx, "Please define scoreid or scorefile", MessageState::WARN, false).await;
        return Ok(());
    }

    ctx.send(poise::CreateReply::default().attachment(serenity::CreateAttachment::bytes(image, "thumbnail.png"))).await?;
    Ok(())
}

/// Either select score id or score file
#[poise::command(slash_command)]
pub async fn title_and_description(
    ctx: Context<'_>,
    #[description = "score id"] scoreid: Option<u64>,
    #[description = "score file"] scorefile: Option<serenity::Attachment>,
) -> Result<(), Error> {
    ctx.defer().await?;
    if scoreid.is_some() {
        let unwrapped_score_id = scoreid.unwrap();
        let score = match osu::get_osu_instance().score(unwrapped_score_id).await {
            Ok(score) => score,
            Err(_) => {
                single_text_response(&ctx, &format!("Score with id {} does not exist", unwrapped_score_id), MessageState::WARN, false).await;
                return Ok(());
            }
        };
        let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap exists");
        let title = youtube_text::generate_title_with_score(&score, &map).await;
        let description = youtube_text::generate_description(score.user_id, score.map_id, Some(&score), None);
        ctx.say(format!("```{}``````{}```", title, description)).await?;
    }
    else if scorefile.is_some() {
        let bytes = scorefile.unwrap().download().await?;
        let replay = match osu_db::Replay::from_bytes(&bytes) {
            Ok(replay) => replay,
            Err(_) => {
                single_text_response(&ctx, "Replay could not be parsed", MessageState::ERROR, false).await;
                return Ok(());
            },
        };
        let timestamp = replay.timestamp.format("%d.%m.%Y at %H:%M").to_string();
        let user = osu::get_osu_instance().user(replay.player_name.as_ref().expect("Expect a username")).await.expect("Player to exist");

        let map: BeatmapExtended = match osu::get_beatmap_from_checksum(&replay.beatmap_hash).await {
            Some(map) => map,
            None => {
                single_text_response(&ctx, "Cannot find map related to the replay", MessageState::WARN, false).await;
                return Ok(());
            },
        };
        let title = youtube_text::generate_title_with_replay(&replay, &map).await;
        let description = youtube_text::generate_description(user.user_id, map.map_id, None, Some(timestamp));

        ctx.say(format!("```{}``````{}```", title, description)).await?;
    }
    else {
        embeds::single_text_response(&ctx, "Please define scoreid or scorefile", MessageState::WARN, false).await;
        return Ok(());
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn render_and_upload (
    ctx: Context<'_>,
    #[description = "score id"] scoreid: Option<u64>,
    #[description = "score file"] scorefile: Option<serenity::Attachment>,
    #[description = "subtitle inside the thumbnail"] subtitle: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let reply = ctx.send(CreateReply::default().embed(embeds::render_and_upload_embed(&"...".into(), false, None, false)?)).await?;

    let cff = ContextForFunctions {
        command_context: Some(ctx),
        reply: Some(reply),
        event_context: None,
        component: None
    };

    if scoreid.is_some() {
        let unwrapped_score_id = scoreid.unwrap();
        let score = match osu::get_osu_instance().score(unwrapped_score_id).await {
            Ok(score) => score,
            Err(_) => {
                cff.edit(single_text_response_embed(&format!("Score with id {} does not exist", unwrapped_score_id), MessageState::WARN), vec![]).await?;
                return Ok(());
            }
        };
        if !score.has_replay {
            cff.edit(single_text_response_embed("Score has no replay to download. Please provide the replay file", MessageState::WARN), vec![]).await?;
            return Ok(());
        }
        if score.mode != rosu::GameMode::Osu {
            cff.edit(single_text_response_embed("Rendering a gamemode other than standard is currently not possible.", MessageState::WARN), vec![]).await?;
            return Ok(());
        }
        let replay = osu::get_osu_instance().replay_raw(score.id).await.unwrap();
        let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap exists");
        let beatmap_hash = map.checksum.as_ref().unwrap().clone();
        let replay_reference = score.id.to_string();
        danser::attach_replay(&beatmap_hash, &replay_reference, &replay).await.unwrap();
        upload::render_and_upload_by_score(&cff, score, map, subtitle).await?;
    }
    else if scorefile.is_some() {
        let bytes = scorefile.unwrap().download().await?;
        let replay = match osu_db::Replay::from_bytes(&bytes) {
            Ok(replay) => replay,
            Err(_) => {
                cff.edit(single_text_response_embed("Replay could not be parsed", MessageState::ERROR), vec![]).await?;
                return Ok(());
            },
        };
        if replay.mode != osu_db::Mode::Standard {
            cff.edit(single_text_response_embed("Rendering a gamemode other than standard is currently not possible.", MessageState::WARN), vec![]).await?;
            return Ok(());
        }
        let user = osu::get_osu_instance().user(replay.player_name.as_ref().expect("Expect a username")).await.expect("Player to exist");

        let map: BeatmapExtended = match osu::get_beatmap_from_checksum(&replay.beatmap_hash).await {
            Some(map) => map,
            None => {
                cff.edit(single_text_response_embed("Cannot find map related to the replay", MessageState::WARN), vec![]).await?;
                return Ok(());
            },
        };
        let beatmap_hash = map.checksum.as_ref().unwrap().clone();
        let replay_reference = replay.replay_hash.as_ref().unwrap().clone();
        danser::attach_replay(&beatmap_hash, &replay_reference, &bytes).await?;
        upload::render_and_upload_by_replay(&cff, replay, map, user, subtitle).await?;
    }
    else {
        embeds::single_text_response(&ctx, "Please define scoreid or scorefile", MessageState::WARN, false).await;
        return Ok(());
    }

    Ok(())
}