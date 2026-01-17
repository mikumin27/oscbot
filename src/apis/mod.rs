use std::{env, fs::{remove_dir_all, remove_file}, io::ErrorKind};

use tokio::{fs::File, io::AsyncWriteExt};

use crate::{Error, discord_helper::{ContextForFunctions, MessageState}, embeds};

mod catboy;
mod nerinyan;

pub mod huismetbenen;
pub mod youtube;

async fn push_mapset(mapset_id: &u32, contents: Vec<u8>) -> Result<(), Error> {
    let osz_path = format!("{}/Songs/{}.osz", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH must exist"), mapset_id);
    remove_file(&osz_path).ok();

    let mut file = match File::create(osz_path).await {
        Ok(file) => file,
        Err(error) => {
            if error.kind() == ErrorKind::AlreadyExists {
                return Ok(());
            }
            return Err(error.into())
        }
    };

    file.write_all(&contents).await?;

    file.flush().await?;
    tracing::info!(mapset_id = mapset_id, "download of osk has finished");
    Ok(())
}

pub async fn download_mapset(cff: &ContextForFunctions<'_>, mapset_id: &u32) -> Result<(), Error> {
    let osz_path = format!("{}/Songs/{}", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH must exist"), mapset_id);
    remove_dir_all(&osz_path).ok();
    match nerinyan::download_mapset(mapset_id).await? {
        Some(skin) => {
            push_mapset(mapset_id, skin).await?;
            return Ok(())
        },
        None => tracing::warn!(mapset_id = mapset_id, "Could not download mapset from nerinyan. Falling back to catboy.best")
    }

    match catboy::download_mapset(mapset_id).await? {
        Some(skin) => {
            push_mapset(mapset_id, skin).await?;
            return Ok(())
        },
        None => tracing::error!(mapset_id = mapset_id, "Could not download mapset from catboy.best. Could not download beatmap.")
    }

    cff.edit(embeds::single_text_response_embed("All mirrors don't have this map", MessageState::ERROR), vec![]).await?;
    Err("Mapset could not be downloaded".into())
}