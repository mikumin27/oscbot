use crate::{Data, Context, Error};
use crate::osu;

async fn error_handler(error: poise::FrameworkError<'_, Data, Error>) {
    println!("Something went horribly wrong: {:?}", error);
}

#[poise::command(slash_command, rename = "test", subcommands("osu_client"), on_error = "error_handler")]
pub async fn bundle(_ctx: Context<'_>, _arg: String) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, on_error = "error_handler")]
pub async fn osu_client(ctx: Context<'_>) -> Result<(), Error> {
    let black_rover = osu::get_osu_instance().beatmap().map_id(1655981).await.expect("Black rover does not exist");
    ctx.say(black_rover.mapset.expect("Mapset does not exist").title).await?;
    Ok(())
}