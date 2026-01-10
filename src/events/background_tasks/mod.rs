use poise::serenity_prelude as serenity;

mod refresh_feed;

pub async fn start_background_tasks(ctx: serenity::Context) {
    loop {
        _ = refresh_feed::refresh_feed(&ctx).await;
        tokio::time::sleep(std::time::Duration::from_secs(180)).await;
    }
}
