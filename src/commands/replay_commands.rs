use poise::serenity_prelude as serenity;
use crate::{Data, Context, Error};
use crate::roles::REPLAY_ROLE;

async fn error_handler(error: poise::FrameworkError<'_, Data, Error>) {
    println!("Something went horribly wrong: {:?}", error);
}

async fn has_replay_role(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => return Ok(false)
    };

    let member = guild_id.member(ctx, ctx.author().id).await?;
    if !member.roles.contains(&REPLAY_ROLE) {
        ctx.say("No permission L").await?;
        return Ok(false);
    }
    Ok(true)
}

#[poise::command(slash_command, rename = "replay", subcommands("generate"), check = "has_replay_role", on_error = "error_handler")]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, subcommands("thumbnail"), check = "has_replay_role", on_error = "error_handler")]
pub async fn generate(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

/// Either select score id or score file
#[poise::command(slash_command)]
pub async fn thumbnail(
    ctx: Context<'_>,
    #[description = "score id"] _scoreid: Option<i64>,
    #[description = "score file"] _scorefile: Option<serenity::Attachment>,
    #[description = "description inside the thumbnail"] _description: String,
) -> Result<(), Error> {
    ctx.say("not implemented yet").await?;
    Ok(())
}