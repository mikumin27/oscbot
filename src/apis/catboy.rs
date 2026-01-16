use reqwest::{Client, header::USER_AGENT};

use crate::{Error};
const URL: &str = "https://catboy.best";

pub async fn download_mapset(mapset_id: &u32) -> Result<Option<Vec<u8>>, Error> {
    tracing::info!(mapset_id = mapset_id, "Downloading osk from catboy.best...");
    let client = Client::new();
    let resp = client
        .get(format!("{}/d/{}", URL, mapset_id)).header(USER_AGENT, "oscbot/0.1 (discord-bot)")
        .send()
        .await
        .unwrap()
        .error_for_status();

    let response = match resp {
        Ok(response) => response,
        Err(error) => {
            println!("{:?}", error);
            return Ok(None);
        }
    };

    Ok(Some(response.bytes().await?.into()))
}