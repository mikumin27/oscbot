use rosu_v2::prelude as rosu;
use poise::serenity_prelude::{self as serenity, Colour, CreateEmbed};

use crate::{apis::huismetbenen, osu};
use crate::{Context, Error};
use crate::discord_helper::MessageState;



static EMBED_COLOR: &[(&MessageState, Colour)] = &[
    (&MessageState::SUCCESS, Colour::new(0x873D48)),
    (&MessageState::WARN, Colour::new(0xFFA53F)),
    (&MessageState::ERROR, Colour::new(0xcc3300)),
    (&MessageState::INFO, Colour::new(0x1434A4)),
];

pub fn get_embed_color(message_state: &MessageState) -> Colour {
    EMBED_COLOR
        .iter()
        .find_map(|(k, v)| (*k == message_state).then_some(*v))
        .expect("State must have color")
}

pub async fn single_text_response(ctx: &Context<'_>, text: &str, message_state: MessageState, ephemeral: bool) {
    let _ = ctx.send(
        poise::CreateReply::default().embed(
            single_text_response_embed(text, message_state)).ephemeral(ephemeral)
    ).await;
}

pub fn single_text_response_embed(text: &str, message_state: MessageState) -> CreateEmbed {
    serenity::CreateEmbed::default().description(text).color(get_embed_color(&message_state))
}

pub async fn score_embed_from_replay_file(replay: &osu_db::Replay, map: &rosu::BeatmapExtended, reason: Option<String>) -> Result<serenity::CreateEmbed, Error> {
    let user = osu::get_osu_instance().user(replay.player_name.as_ref().expect("Expect a username")).await.expect("Player to exist");
    let result = huismetbenen::calculate_score_by_replay(replay, map).await;
    let hits = format!("{}/{}/{}/{}", replay.count_300, replay.count_100, replay.count_50, replay.count_miss);
    let mods = osu::formatter::convert_osu_db_to_mod_array(replay.mods).join("");
    score_embed(map, &user, Some(replay.online_score_id), replay.score, result.accuracy, hits, replay.max_combo as u32, mods, Some(result.pp), rosu::GameMode::from(replay.mode.raw()), reason).await
}

pub async fn score_embed_from_score(score: &rosu::Score, map: &rosu::BeatmapExtended, reason: Option<String>) -> Result<serenity::CreateEmbed, Error> {
    let user = score.get_user(osu::get_osu_instance()).await.expect("User has not been found");
    let hits = osu::formatter::osu_hits(&score.statistics, &score.mode);
    let mods = osu::formatter::mods_string(&score.mods);
    score_embed(map, &user, Some(score.id), score.score, score.accuracy, hits, score.max_combo, mods, score.pp, score.mode, reason).await
}

async fn score_embed(
    map: &rosu::BeatmapExtended,
    user: &rosu::UserExtended,
    score_id: Option<u64>,
    score: u32,
    accuracy: f32,
    hits: String,
    max_combo: u32,
    mods: String,
    pp: Option<f32>,
    mode: rosu::GameMode,
    reason: Option<String>,
) -> Result<serenity::CreateEmbed, Error> {
    let mapset = map.mapset.as_ref().expect("Mapset has not been found");
    let embed = serenity::CreateEmbed::default();
    let title = osu::formatter::map_title(&map);
    let mut author = serenity::CreateEmbedAuthor::new(format!("Score done by {} - {}", user.username, osu::formatter::game_mode_name(mode)));

    match score_id {
        Some(score_id) => {
            author = author.url(osu::formatter::score_url(&score_id));
        },
        _ => (),
    };

    Ok(embed.author(author).color(get_embed_color(&MessageState::SUCCESS))
         .title(title)
         .url(map.url.to_string())
         .thumbnail(user.avatar_url.clone())
         .image(mapset.covers.card.clone())
         .field("Score:", score.to_string(), true)
         .field("Accuracy:", format!("{:.2}",accuracy), true)
         .field("Hits:", hits, true)
         .field("Combo:", max_combo.to_string() + "x", true)
         .field("Mods:", mods, true)
         .field("PP:", format!("{:.2}", pp.unwrap_or(0.0)), true)
         .field("Reason:", reason.unwrap_or("No reason provided".into()), false))
}

pub fn render_and_upload_embed(
    title: &String,
    preparation_finished: bool,
    render_replay: Option<String>,
    video_uploaded: bool
) -> Result<serenity::CreateEmbed, Error> {
    let embed = serenity::CreateEmbed::default();
    let author = serenity::CreateEmbedAuthor::new("Upload progress:");

    let mut replay_rendered: bool = false;
    let render_replay_string: String = match render_replay {
        Some(string) => {
            match string.as_str() {
                "100%" => {
                    replay_rendered = true;
                    "done".to_string()
                }
                _ => string
            }
        },
        None => if preparation_finished { "0%".into() } else { "awaiting".into() }
    };

    let upload_video_string = if !replay_rendered {
        "awaiting"
    }
    else {
        if video_uploaded { "done" } else { "ongoing" }
    };

    Ok(embed.author(author).color(get_embed_color(&MessageState::SUCCESS))
         .title(title)
         .field("Prepare replay:", if preparation_finished {"Finished"} else {"Ongoing..."}, false)
         .field("Render replay:", render_replay_string, false)
         .field("Upload Video:", upload_video_string, false))
}

pub fn upload_result_embed (
    title: &String,
    youtube_id: &String,
    title_too_long: bool,
) -> Result<serenity::CreateEmbed, Error> {
    let mut embed = serenity::CreateEmbed::default();
    let author = serenity::CreateEmbedAuthor::new("Render and upload");

    embed = embed.author(author)
            .color(get_embed_color(&MessageState::SUCCESS))
            .title(title)
            .description(format!("Video has been uploaded successfully: https://studio.youtube.com/video/{}/edit", youtube_id));

    if title_too_long {
        embed = embed.field("Warning: The title was too long. Please adjust accordingly and set it yourself.", title, false);
    }
    Ok(embed)
}

pub fn suggestion_approved_embed (
    title: &String,
) -> Result<serenity::CreateEmbed, Error> {
    let mut embed = serenity::CreateEmbed::default();
    let author = serenity::CreateEmbedAuthor::new("Suggestion");

    embed = embed.author(author)
            .color(get_embed_color(&MessageState::SUCCESS))
            .title("✅ Your suggestion has been approved!")
            .description(format!("Score: {}", title));

    Ok(embed)
}

pub fn suggestion_declined_embed (
    title: &String,
) -> Result<serenity::CreateEmbed, Error> {
    let mut embed = serenity::CreateEmbed::default();
    let author = serenity::CreateEmbedAuthor::new("Suggestion");

    embed = embed.author(author)
            .color(get_embed_color(&MessageState::SUCCESS))
            .title("❌ Your suggestion has been declined!")
            .description(format!("Score: {}", title));

    Ok(embed)
}