use crate::{Data, Error};

mod test_commands;
mod replay_commands;
mod suggest_commands;

pub fn slash_commands_bundle() -> Vec<poise::Command<Data, Error>> {
    return vec![
            replay_commands::bundle(),
            suggest_commands::bundle(),
            test_commands::bundle()
    ]
}