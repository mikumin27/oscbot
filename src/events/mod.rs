use poise::FrameworkContext;
use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub mod background_tasks;
mod message_event;
mod button_actions;

pub fn handle_events<'a>(
    framework: FrameworkContext<'a, Data, Error>,
    event: &'a serenity::FullEvent,
) -> poise::BoxFuture<'a, Result<(), Error>> {
    let ctx = framework.serenity_context;
    Box::pin(async move {
        if let serenity::FullEvent::Message { new_message } = event {
            message_event::handle_message(&ctx, &new_message).await?;
        }
        if let serenity::FullEvent::InteractionCreate { interaction } = event {
            if let serenity::Interaction::Component(component) = interaction {
                button_actions::handle_click(&ctx, component).await?;
            }
        }
        Ok(())
    })
}