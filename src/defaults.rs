use std::sync::LazyLock;
use poise::serenity_prelude as serenity;


pub static REPLAY_ROLE: LazyLock<serenity::RoleId> = LazyLock::new(|| {
    let id: u64 = std::env::var("OSC_BOT_REPLAY_ADMIN_ROLE")
        .expect("OSC_BOT_REPLAY_ADMIN_ROLE not set")
        .parse()
        .expect("OSC_BOT_REPLAY_ADMIN_ROLE must be u64");
    serenity::RoleId::new(id)
});

pub static SERVER: LazyLock<serenity::GuildId> = LazyLock::new(|| {
    let id: u64 = std::env::var("OSC_BOT_DISCORD_SERVER")
        .expect("OSC_BOT_DISCORD_SERVER not set")
        .parse()
        .expect("REPLAYOSC_BOT_DISCORD_SERVER_ADMIN_ROLE must be u64");
    serenity::GuildId::new(id)
});

pub static SUGGESTIONS_CHANNEL: LazyLock<serenity::ChannelId> = LazyLock::new(|| {
    let id: u64 = std::env::var("OSC_BOT_REQUEST_CHANNEL")
        .expect("OSC_BOT_REQUEST_CHANNEL not set")
        .parse()
        .expect("OSC_BOT_REQUEST_CHANNEL must be u64");
    serenity::ChannelId::new(id)
});
