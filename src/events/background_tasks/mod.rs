use poise::serenity_prelude as serenity;

use crate::events::background_tasks::refresh_feed::run_refresh_feed;

mod refresh_feed;

pub fn start_background_tasks(ctx: &serenity::Context) {
    tokio::spawn(run_refresh_feed(ctx.clone()));
}