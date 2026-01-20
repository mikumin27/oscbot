use poise::serenity_prelude as serenity;
use tracing_subscriber::{EnvFilter, fmt};

use crate::events::background_tasks;

mod migrations;
mod db;

mod embeds;
mod apis;
mod osu;
mod emojis;
mod defaults;
mod commands;
mod events;
mod generate;
mod discord_helper;

#[derive(Debug)]
struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;


fn init_logging() {
    // RUST_LOG=info,mycrate=debug,hyper=warn
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false) // hide module path unless you want it
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .compact()
        .init();

    tracing::info!("logging initialized (set RUST_LOG to adjust verbosity)");
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_logging();
    tracing::info!("starting up...");
    osu::initialize_osu().await.unwrap();
    tracing::info!("osu!api initialized!");
    tracing::info!("inserting migrations!");
    migrations::update_migrations().await.unwrap();
    db::init_db().await.unwrap();
    tracing::info!("db initialized!");

    
    let token = std::env::var("OSC_BOT_DISCORD_TOKEN").expect("missing OSC_BOT_DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::slash_commands_bundle(),
            event_handler: |ctx, event, framework, data| {
                events::handle_events(&ctx, &event, &framework, &data)
            },
            on_error: |error| {
                Box::pin(discord_helper::handle_error(error))
            },
            command_check: Some(|ctx| Box::pin(discord_helper::global_check(ctx))),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx.clone(), &framework.options().commands).await?;
                background_tasks::start_background_tasks(ctx);
                tracing::info!("The bot is ready to use!");
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
