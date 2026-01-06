use crate::{Data, Error};

mod dev_commands;
mod replay_commands;
mod suggest_commands;
mod skin_commands;
mod admin_commands;

pub fn slash_commands_bundle() -> Vec<poise::Command<Data, Error>> {
    if cfg!(debug_assertions) {
        return vec![
            dev_commands::bundle(),
            replay_commands::bundle(),
            suggest_commands::bundle(),
            skin_commands::bundle(),
            admin_commands::bundle(),
    ]
    }
    return vec![
            replay_commands::bundle(),
            suggest_commands::bundle(),
            skin_commands::bundle(),
            admin_commands::bundle(),
    ]
}
