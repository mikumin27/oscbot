use poise::serenity_prelude::{ self as serenity, ComponentInteraction, CreateAttachment, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, EditAttachments, EditMessage};
use rosu_v2::prelude::BeatmapExtended;
use crate::discord_helper::{ContextForFunctions, MessageState, user_has_replay_role};
use crate::osu::get_osu_instance;
use crate::{Error, embeds, osu};
use crate::generate::{danser, thumbnail, upload, youtube_text};

enum ScoreType {
    ScoreId,
    ReplayFile,
}

struct ScoreMapping {
    reference: String,
    map: BeatmapExtended,
    score_type: ScoreType,
    requesting_user: serenity::User
}

impl ScoreMapping {
    async fn new(ctx: &serenity::Context, contents: [&str; 4]) -> ScoreMapping {
        let user_id: u64 = contents[3].parse().expect("not a u64");
        let user = ctx.http.get_user(user_id.into()).await.unwrap();
        let map = get_osu_instance().beatmap().map_id(contents[2].parse().unwrap()).await.unwrap();
        ScoreMapping {
            reference: contents[1].to_string(),
            map: map,
            score_type: match contents[0] {
                "scoreid" => ScoreType::ScoreId,
                "replayfile" => ScoreType::ReplayFile,
                _ => ScoreType::ScoreId
            },
            requesting_user: user
        }
    }
}

pub async fn handle_click(ctx: &serenity::Context, component: &ComponentInteraction) -> Result<(), Error> {
    tracing::info!(identifier = component.data.custom_id, "Interaction has been initiated");
    let mut parts: std::str::Split<'_, char> = component.data.custom_id.split(':');

    let identifier = parts.next().unwrap();
    let data: Vec<&str> = parts.collect();

    if !user_has_replay_role(ctx, &component.user).await.unwrap() {
        tracing::warn!(user = component.user.display_name(), "User tried to use interaction without permission");
        _ = component.create_response(ctx, 
            serenity::CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::default().embed(
                    CreateEmbed::default().description("No permission L").color(embeds::get_embed_color(&MessageState::INFO))
                ).ephemeral(true)
            )
        ).await?;
        return Ok(());
    }

    let mut message = component.message.clone();

    let _ = match identifier {
        "approveWithUpload" => {
            let score = ScoreMapping::new(ctx, data.try_into().expect("Data must have 3 values")).await;
            let title = match score.score_type {
                ScoreType::ScoreId => upload_score_by_score(ctx, component, &score).await.unwrap(),
                ScoreType::ReplayFile => upload_score_by_replay(ctx, component, &score).await.unwrap(),
            };
            score.requesting_user.dm(ctx, CreateMessage::default().add_embed(embeds::suggestion_approved_embed(&title)?)).await?;
            message.edit(ctx, EditMessage::default().components(vec![])).await?;
            
        },
        "approveNoUpload" => {
            component.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::default().content("Loading content..."))).await?;
            let score = ScoreMapping::new(ctx, data.try_into().expect("Data must have 3 values")).await;
            let title = match score.score_type {
                ScoreType::ScoreId => get_score_metadata_by_score(ctx, component, &score).await.unwrap(),
                ScoreType::ReplayFile => get_score_metadata_by_replay(ctx, component, &score).await.unwrap(),
            };
            score.requesting_user.dm(ctx, CreateMessage::default().add_embed(embeds::suggestion_approved_embed(&title)?)).await?;
            message.edit(ctx, EditMessage::default().components(vec![])).await?;
            
        },
        "decline" => {
            let score = ScoreMapping::new(ctx, data.try_into().expect("Data must have 3 values")).await;
            let title = match score.score_type {
                ScoreType::ScoreId => {
                    let score = osu::get_osu_instance().score(score.reference.parse().unwrap()).await?;
                    let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await?;
                    youtube_text::generate_title_with_score(&score, &map).await
                }
                ScoreType::ReplayFile => {
                    let replay = danser::get_replay(&score.reference, &score.map.checksum.as_ref().unwrap()).await?;
                    youtube_text::generate_title_with_replay(&replay, &score.map).await
                }
            };
            score.requesting_user.dm(ctx, CreateMessage::default().add_embed(embeds::suggestion_declined_embed(&title)?)).await?;
            message.edit(ctx, EditMessage::default().components(vec![])).await?;
        }
        _ => return Err("Identifier of component has not been found".into())
    };
    Ok(())
}

async fn upload_score_by_replay(ctx: &serenity::Context, component: &serenity::ComponentInteraction, score: &ScoreMapping) -> Result<String, Error> {
    let mut cff = ContextForFunctions {
        command_context: None,
        reply: None,
        event_context: Some(ctx),
        component: Some(component)
    };

    let beatmap_hash = score.map.checksum.as_ref().unwrap();
    let replay = danser::get_replay(&score.reference, &beatmap_hash).await.unwrap();

    let user = osu::get_osu_instance().user(replay.player_name.as_ref().unwrap()).await.unwrap();

    let title = youtube_text::generate_title_with_replay(&replay, &score.map).await;
    cff.send(embeds::render_and_upload_embed(&"...".to_string(), false, None, false)?).await?;
    upload::render_and_upload_by_replay(&cff, replay, score.map.clone(),  user,None).await?;
    Ok(title)
}

async fn upload_score_by_score(ctx: &serenity::Context, component: &serenity::ComponentInteraction, score: &ScoreMapping) -> Result<String, Error> {
    let mut cff = ContextForFunctions {
        command_context: None,
        reply: None,
        event_context: Some(ctx),
        component: Some(component)
    };

    cff.send(embeds::render_and_upload_embed(&"...".to_string(), false, None, false)?).await?;
    let score_id: u64 = score.reference.parse().unwrap();
    let score = osu::get_osu_instance().score(score_id).await.expect("Score must exist");
    let replay_bytes = osu::get_osu_instance().replay_raw(score_id).await.unwrap();
    let map = osu::get_osu_instance().beatmap().map_id(score.map_id).await.expect("Beatmap must exist");
    let title = youtube_text::generate_title_with_score(&score, &map).await;
    danser::attach_replay(&map.checksum.as_ref().unwrap(), &score_id.to_string(), &replay_bytes).await?;
    upload::render_and_upload_by_score(&cff, score, map, None).await?;
    Ok(title)
}

async fn get_score_metadata_by_replay(ctx: &serenity::Context, component: &serenity::ComponentInteraction, score: &ScoreMapping) -> Result<String, Error> {
    let beatmap_hash = score.map.checksum.as_ref().unwrap();
    let replay = danser::get_replay(&score.reference, &beatmap_hash).await?;
    
    let replay_file = danser::get_replay_file(&score.reference, &beatmap_hash).await?;
    let user = osu::get_osu_instance().user(replay.player_name.as_ref().unwrap()).await?;
    let map = osu::get_osu_instance().beatmap().checksum(replay.replay_hash.as_ref().unwrap()).await?;

    let timestamp = replay.timestamp.format("%d.%m.%Y at %H:%M").to_string();
    let title = youtube_text::generate_title_with_replay(&replay, &map).await;
    let description = youtube_text::generate_description(user.user_id, map.map_id, None, Some(timestamp));
    let thumbnail = thumbnail::generate_thumbnail_from_replay_file(&replay, &map, &"".to_string()).await;
    
    component.edit_response(ctx,serenity::EditInteractionResponse::default()
        .content(format!("```{}``````{}```", title, description))
        .attachments(EditAttachments::default()
            .add(CreateAttachment::file(&replay_file, "replay.osr").await?)
            .add(CreateAttachment::bytes(thumbnail, "thumbnail.jpeg"))
        )
    ).await?;
    Ok(title)
}

async fn get_score_metadata_by_score(ctx: &serenity::Context, component: &serenity::ComponentInteraction, score: &ScoreMapping) -> Result<String, Error> {    
    let map = score.map.clone();
    let score_id: u64 = score.reference.parse().unwrap();
    let score = osu::get_osu_instance().score(score_id).await.expect("Score must exist");
    let replay_file = osu::get_osu_instance().replay_raw(score_id).await?;
    

    let title = youtube_text::generate_title_with_score(&score, &map).await;
    let description = youtube_text::generate_description(score.user_id, map.map_id, Some(&score), None);
    let thumbnail = thumbnail::generate_thumbnail_from_score(&score, &map, &"".to_string()).await;
    
    component.edit_response(ctx,serenity::EditInteractionResponse::default()
        .content(format!("```{}``````{}```", title, description))
        .attachments(EditAttachments::default()
            .add(CreateAttachment::bytes(replay_file, "replay.osr"))
            .add(CreateAttachment::bytes(thumbnail, "thumbnail.jpeg"))
        )
    ).await?;
    Ok(title)
}