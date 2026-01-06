use std::env;
use std::fs::remove_file;
use std::io::ErrorKind;

use futures_util::StreamExt;

use reqwest::Client;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::{Error, embeds};
use crate::discord_helper::{ContextForFunctions, MessageState};
const URL: &str = "https://api.nerinyan.moe";

pub async fn download_mapset(cff: &ContextForFunctions<'_>, mapset_id: &u32) -> Result<(), Error> {
    tracing::info!(mapset_id = mapset_id, "Downloading osk from nerinyan...");
    let client = Client::new();
    let resp = client
        .get(format!("{}/d/{}", URL, mapset_id))
        .send()
        .await
        .unwrap()
        .error_for_status();

    let response = match resp {
        Ok(response) => response,
        Err(_) => {
            tracing::error!(mapset_id = mapset_id, "Could not download mapset from nerinyan");
            cff.edit(embeds::single_text_response_embed("nerinyan error: Mapset has not been found", MessageState::ERROR), vec![]).await?;
            return Ok(());
        }
    };
    let osz_path = format!("{}/Songs/{}.osz", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH must exist"), mapset_id);
    _ = remove_file(&osz_path);
    let mut file = match File::create(osz_path).await {
        Ok(file) => file,
        Err(error) => {
            if error.kind() == ErrorKind::AlreadyExists {
                return Ok(());
            }
            return Err(error.into())
        }
    };

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    tracing::info!(mapset_id = mapset_id, "download of osk has finished");
    Ok(())
}