use poise::{CreateReply, serenity_prelude::{self as serenity, CreateEmbed, CreateEmbedAuthor}};
use url::Url;

use crate::{Context, Error, apis::osc_web, db, discord_helper::MessageState, embeds::single_text_response, osu};

const OSC_WEB_HOME: &str = "https://skins.sulej.net/";

fn skin_doc_url(pick: &osc_web::PickEntry) -> String {
    let mut u = Url::parse("https://skins.sulej.net").unwrap();
    if let Ok(mut segs) = u.path_segments_mut() {
        if pick.is_community() {
            // The community skin lives at its own route, not a user profile.
            segs.push("osc-skins").push(&pick.dir_name);
        } else {
            segs.push("users")
                .push(&pick.owner_osu_id.unwrap_or(0).to_string())
                .push(&pick.dir_name);
        }
    }
    u.to_string()
}

const PICK_MODIFIERS: &[&str] = &[
    "DEFAULT", "NM", "HD", "DT", "HR", "EZ", "HDDT", "HDHR",
];

#[poise::command(
    slash_command,
    rename = "skin",
    subcommands("set", "get"),
    required_permissions = "SEND_MESSAGES"
)]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
    Ok(())
}

/// Tell the user how to set their picks on the website.
#[poise::command(slash_command)]
pub async fn set(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let message = format!(
        "**Skin picking lives on the website now.**\n\
         \n\
         1. Open <{OSC_WEB_HOME}> and log in with osu!\n\
         2. Browse the community and find a skin you like, on any user's profile.\n\
         3. On the skin you want, hit the bot icon to mark it as your preferred render skin (tick one or more mod combinations).\n\
         \n\
         The bot will use whatever you pick the next time it renders one of your replays."
    );
    single_text_response(&ctx, &message, MessageState::INFO, true).await;
    Ok(())
}

/// Show this user's current render-pick layout per mod combination.
#[poise::command(slash_command)]
pub async fn get(
    ctx: Context<'_>,
    #[description = "leave empty to show your own picks"] member: Option<serenity::Member>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let username = match &member {
        Some(m) => m.display_name().to_string(),
        None => match ctx.author_member().await {
            Some(m) => m.display_name().to_string(),
            None => ctx.author().name.clone(),
        },
    };
    let user_id: i64 = match &member {
        Some(m) => m.user.id.into(),
        None => ctx.author().id.into(),
    };

    let player = match osu::get_osu_instance().user(&username).await {
        Ok(u) => u,
        Err(_) => {
            single_text_response(
                &ctx,
                "I couldn't match that Discord name to an osu! user. Make sure your Discord display name matches your osu! username.",
                MessageState::WARN,
                false,
            )
            .await;
            return Ok(());
        }
    };

    db::get_user_by_discord_id_or_create(user_id, player.user_id as i32).await?;

    let picks = match osc_web::get_user_picks(player.user_id as i64).await {
        Ok(p) => p,
        Err(e) => {
            single_text_response(
                &ctx,
                &format!("Couldn't reach skins.sulej.net: {}", e),
                MessageState::WARN,
                false,
            )
            .await;
            return Ok(());
        }
    };

    let mut lines: Vec<String> = Vec::new();
    for slot in PICK_MODIFIERS {
        match picks.get(*slot).and_then(|v| v.as_ref()) {
            // Own picks and the community skin render plain (name + link); only
            // cross-user picks get a "from osu! id …" attribution.
            Some(p) if p.is_community() || p.owner_osu_id == Some(player.user_id as i64) => {
                let url = skin_doc_url(p);
                lines.push(format!("**{slot}** — [{}]({})", p.dir_name, url));
            }
            Some(p) => {
                let url = skin_doc_url(p);
                lines.push(format!(
                    "**{slot}** — [{}]({}) *(from osu! id {})*",
                    p.dir_name,
                    url,
                    p.owner_osu_id.unwrap_or(0)
                ));
            }
            None => lines.push(format!("**{slot}** — *(no pick)*")),
        }
    }

    let embed = CreateEmbed::default()
        .author(CreateEmbedAuthor::new(format!(
            "{}'s render picks",
            username
        )))
        .description(lines.join("\n"))
        .url(format!("https://skins.sulej.net/users/{}/picks", player.user_id));
    ctx.send(CreateReply::default().embed(embed)).await.unwrap();
    Ok(())
}
