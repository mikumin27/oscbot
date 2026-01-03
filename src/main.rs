use poise::serenity_prelude as serenity;

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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    osu::initialize_osu().await.unwrap();
    firebase::initialize_firebase().await.unwrap();

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