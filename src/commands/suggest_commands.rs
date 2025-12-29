use poise::serenity_prelude as serenity;
use crate::{Data, Context, Error};

async fn error_handler(error: poise::FrameworkError<'_, Data, Error>) {
    println!("Something went horribly wrong: {:?}", error);
}

#[poise::command(slash_command, rename = "suggest", subcommands("score"), required_permissions = "SEND_MESSAGES", on_error = "error_handler")]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

/// Either submit score id or score file
#[poise::command(slash_command)]
pub async fn score(
    ctx: Context<'_>,
    #[description = "score id"] _scoreid: Option<i64>,
    #[description = "score file"] _scorefile: Option<serenity::Attachment>,
    #[description = "reason"] _reason: Option<String>,
) -> Result<(), Error> {
    ctx.say("not implemented yet").await?;
    Ok(())
}