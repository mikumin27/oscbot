use std::env;

use serde::Deserialize;

use crate::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct OscWebSkin {
    pub dir_name: String,
    #[serde(rename = "osk_url")]
    pub url_path: String,
    #[serde(default)]
    pub skin_name: Option<String>,
    #[serde(rename = "skin_owner_osu_id", default)]
    pub owner_osu_id: Option<i64>,
    /// `"user"` or `"community"`. Absent on the pre-re-model API → treated as a
    /// user pick (with `owner_osu_id`).
    #[serde(rename = "skin_owner_kind", default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub matched_modifier: Option<String>,
}

impl OscWebSkin {
    /// Whether this skin is the OSC community skin (vs a member's repo).
    pub fn is_community(&self) -> bool {
        self.owner_kind.as_deref() == Some("community")
    }

    pub fn url(&self) -> String {
        if self.url_path.starts_with("http://") || self.url_path.starts_with("https://") {
            return self.url_path.clone();
        }
        let base = base_url();
        format!("{}{}", base.trim_end_matches('/'), self.url_path)
    }
}

fn base_url() -> String {
    env::var("OSC_WEB_BASE_URL").unwrap_or_else(|_| "https://skins.sulej.net".to_string())
}

fn bot_token() -> Result<String, Error> {
    env::var("OSC_WEB_BOT_TOKEN")
        .map_err(|_| "OSC_WEB_BOT_TOKEN env var is not set".into())
}

pub async fn fetch_osc_skin() -> Result<OscWebSkin, Error> {
    let token = bot_token()?;
    let url = format!("{}/api/bot/osc-skin", base_url().trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .error_for_status()?;
    let skin: OscWebSkin = resp.json().await?;
    Ok(skin)
}

pub async fn skin_pick(osu_id: i64, mods: &[String]) -> Result<Option<OscWebSkin>, Error> {
    let token = bot_token()?;
    let url = format!(
        "{}/api/bot/users/{}/picks/resolve?mods={}",
        base_url().trim_end_matches('/'),
        osu_id,
        mods.join(","),
    );
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let resp = resp.error_for_status()?;
    let skin: OscWebSkin = resp.json().await?;
    Ok(Some(skin))
}

#[derive(Debug, Clone, Deserialize)]
pub struct PickEntry {
    /// None for community picks (and on the pre-re-model API it's always set).
    #[serde(default)]
    pub owner_osu_id: Option<i64>,
    #[serde(default)]
    pub owner_kind: Option<String>,
    pub dir_name: String,
}

impl PickEntry {
    pub fn is_community(&self) -> bool {
        self.owner_kind.as_deref() == Some("community")
    }
}

pub async fn get_user_picks(
    osu_id: i64,
) -> Result<std::collections::BTreeMap<String, Option<PickEntry>>, Error> {
    let url = format!(
        "{}/api/users/{}/picks",
        base_url().trim_end_matches('/'),
        osu_id,
    );
    let mut req = reqwest::Client::new().get(&url);
    if let Ok(token) = bot_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let body: std::collections::BTreeMap<String, Option<PickEntry>> =
        req.send().await?.error_for_status()?.json().await?;
    Ok(body)
}
