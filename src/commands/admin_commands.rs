use poise::{CreateReply, serenity_prelude::{self as serenity, CreateEmbed, Mentionable}};

use crate::{Context, Error, discord_helper::{MessageState, user_has_replay_role}, embeds::single_text_response, firebase};

async fn has_replay_role(ctx: Context<'_>) -> Result<bool, Error> {
    if !user_has_replay_role(ctx, ctx.author()).await.unwrap() {
        single_text_response(&ctx, "No permission L", MessageState::INFO, true).await;
        return Ok(false);
    }
    Ok(true)
}


#[poise::command(slash_command, rename = "admin", subcommands("blacklist"), check="has_replay_role")]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, subcommands("add", "remove", "list"))]
pub async fn blacklist(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    user: serenity::Member
) -> Result<(), Error> {

    firebase::user::add_to_blacklist(&user.user.id.to_string()).await;
    single_text_response(&ctx, &format!("User {} has been blacklisted", user.mention().to_string()), MessageState::SUCCESS, false).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    user: serenity::Member
) -> Result<(), Error> {
    firebase::user::remove_from_blacklist(&user.user.id.to_string()).await;
    single_text_response(&ctx, &format!("User {} has been removed from the blacklist", user.mention().to_string()), MessageState::SUCCESS, false).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let blacklist = match firebase::user::get_blacklist().await {
        Some(blacklist) => blacklist,
        None => {
            single_text_response(&ctx, "The blacklist is empty", MessageState::INFO, false).await;
            return Ok(())
        } 
    };

    let mut blacklist_content = "".to_string();
    for (user, _) in blacklist {
        blacklist_content = format!("{}<@{}>\n", blacklist_content, user);
    }

    let blacklist_embed = CreateEmbed::default().title("Blacklist").description(blacklist_content);
    ctx.send(CreateReply::default().embed(blacklist_embed)).await?;
    Ok(())
}