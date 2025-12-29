use poise::FrameworkContext;
use poise::serenity_prelude as serenity;

use crate::{Data, Error};

mod message_event;

pub fn handle_events<'a>(
    ctx: &'a serenity::Context,
    event: &'a serenity::FullEvent,
    _framework: &FrameworkContext<'a, Data, Error>,
    _data: &Data
) -> poise::BoxFuture<'a, Result<(), Error>> {
    Box::pin(async move {
        if let serenity::FullEvent::Message { new_message } = event {
            message_event::handle_message(&ctx, &new_message).await?;
        }
        Ok(())
    })
}