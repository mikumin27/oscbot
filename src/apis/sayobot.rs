use crate::Error;

pub async fn download_mapset(mapset_id: &u32) -> Result<Option<Vec<u8>>, Error> {
    let url = format!("https://txy1.sayobot.cn/beatmaps/download/full/{mapset_id}?server=auto");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10)) 
        .user_agent("oscbot/0.1 (discord-bot)") 
        .build()?;

    let response = match client.get(url).send().await?.error_for_status() {
        Ok(response) => response,
        Err(_) => return Ok(None)
    };

    let bytes = response.bytes().await?;

    Ok(Some(bytes.to_vec()))
}