use poise::serenity_prelude as serenity;
use tracing_subscriber::{EnvFilter, fmt};

mod embeds;
mod firebase;
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
        .compact()
        .init();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_logging();
    tracing::trace!("starting up...");
    osu::initialize_osu().await.unwrap();
    tracing::trace!("osu!api initialized!");
    firebase::initialize_firebase().await.unwrap();
    tracing::trace!("firebase initialized!");

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
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}