use poise::serenity_prelude::{self as serenity};

use crate::{Error};
use crate::emojis;

pub async fn handle_message(ctx: &serenity::Context, new_message: &serenity::Message) -> Result<(), Error> {
    if new_message.author.bot {
        return Ok(());
    }
    
    if new_message.content == "sata andagi" {
        new_message.react(ctx, emojis::SATA_ANDAGI).await?;
    }
    
    Ok(())
}