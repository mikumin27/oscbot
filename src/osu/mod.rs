use std::{sync::OnceLock};

use rosu_v2::prelude as osu;

use crate::{Error};

static OSU: OnceLock<osu::Osu> = OnceLock::new();

pub async fn initialize_osu() -> Result<(), Error> {
    let client_id: u64 = std::env::var("OSC_BOT_CLIENT_ID")
    .expect("Client id must be defined")
    .parse()
    .expect("CLient id must be integer");
    let client_secret = std::env::var("OSC_BOT_CLIENT_SECRET").expect("Client secret must be defined");

    match OSU.set(osu::Osu::new(client_id, client_secret).await.unwrap()) {
        Ok(_) => return Ok(()),
        Err(_) => panic!("osu client could not be initialized"),
    };
}

pub fn get_osu_instance() -> &'static osu::Osu {
    OSU.get().expect("OSU is not initialized yet")
}