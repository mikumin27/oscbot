use std::sync::LazyLock;
use poise::serenity_prelude as serenity;


pub static REPLAY_ROLE: LazyLock<serenity::RoleId> = LazyLock::new(|| {
    let id: u64 = std::env::var("REPLAY_ADMIN_ROLE")
        .expect("REPLAY_ADMIN_ROLE not set")
        .parse()
        .expect("REPLAY_ADMIN_ROLE must be u64");
    serenity::RoleId::new(id)
});