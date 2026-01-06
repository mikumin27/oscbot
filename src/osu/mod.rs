use std::{sync::OnceLock};

use rosu_v2::prelude as rosu;

use crate::{Error};

pub mod formatter;

static OSU: OnceLock<rosu::Osu> = OnceLock::new();

pub async fn initialize_osu() -> Result<(), Error> {
    let client_id: u64 = std::env::var("OSC_BOT_CLIENT_ID")
    .expect("Client id must be defined")
    .parse()
    .expect("CLient id must be integer");
    let client_secret = std::env::var("OSC_BOT_CLIENT_SECRET").expect("Client secret must be defined");

    match OSU.set(rosu::Osu::new(client_id, client_secret).await.unwrap()) {
        Ok(_) => return Ok(()),
        Err(_) => {panic!("osu client could not be initialized")},
    };
}

pub fn get_osu_instance() -> &'static rosu::Osu {
    OSU.get().expect("OSU is not initialized yet")
}

pub async fn get_beatmap_from_checksum(checksum: &Option<String>) -> Option<rosu::BeatmapExtended> {
    let checksum_for_searching = match checksum {
        Some(checksum) => checksum,
        None => return None
    };       
    match get_osu_instance().beatmap().checksum(checksum_for_searching).await {
        Ok(map) => return Some(map),
        _ => {
            return None;
        },
    };

}