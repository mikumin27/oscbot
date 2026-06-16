use std::env;

use serde::Deserialize;
use url::Url;

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
    /// `"user"` or `"community"`.
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

    /// Frontend page for this skin: the community collection detail
    /// (`/osc-skins/{dir}`) or the owner's profile skin (`/users/{id}/{dir}`).
    /// `dir_name` is pushed as a path segment so spaces, `+`, and unicode are
    /// percent-encoded.
    pub fn doc_url(&self) -> String {
        let mut u = Url::parse(&base_url())
            .unwrap_or_else(|_| Url::parse("https://skins.sulej.net").unwrap());
        if let Ok(mut segs) = u.path_segments_mut() {
            if self.is_community() {
                segs.push("osc-skins").push(&self.dir_name);
            } else if let Some(id) = self.owner_osu_id {
                segs.push("users").push(&id.to_string()).push(&self.dir_name);
            }
        }
        u.to_string()
    }
}

fn base_url() -> String {
    env::var("OSC_WEB_BASE_URL").unwrap_or_else(|_| "https://skins.sulej.net".to_string())
}

fn bot_token() -> Result<String, Error> {
    env::var("OSC_WEB_BOT_TOKEN").map_err(|_| "OSC_WEB_BOT_TOKEN env var is not set".into())
}

/// GET `url`, attaching the bot token when it points at osc-web. The media routes
/// require it now that download URLs are no longer pre-signed; external/legacy
/// URLs (full http(s) that aren't ours) are fetched plain.
pub async fn download_bytes(url: &str) -> Result<Vec<u8>, Error> {
    let base = base_url();
    let mut req = reqwest::Client::new().get(url);
    if url.starts_with(&base)
        && let Ok(token) = bot_token()
    {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let resp = req.send().await?.error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

#[derive(Debug, Clone, Deserialize)]
struct CommunityPage {
    skins: Vec<CommunitySkinEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommunitySkinEntry {
    dir_name: String,
    #[serde(default)]
    skin_name: Option<String>,
    #[serde(default)]
    osk_url: Option<String>,
}

/// The OSC community skin (first entry of its collection), installed at startup
/// as danser's render fallback.
pub async fn fetch_osc_skin() -> Result<OscWebSkin, Error> {
    let token = bot_token()?;
    let url = format!("{}/api/community/osc", base_url().trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .error_for_status()?;
    let page: CommunityPage = resp.json().await?;
    let entry = page
        .skins
        .into_iter()
        .next()
        .ok_or_else(|| -> Error { "no skin in the osc community collection".into() })?;
    let url_path = entry
        .osk_url
        .ok_or_else(|| -> Error { "osc community skin has no osk".into() })?;
    Ok(OscWebSkin {
        dir_name: entry.dir_name,
        url_path,
        skin_name: entry.skin_name,
        owner_osu_id: None,
        owner_kind: Some("community".to_string()),
        matched_modifier: None,
    })
}

/// Ordered modifier slots to try for a play; the caller takes the first slot the
/// user has a pick for. Resolution lives here now (osc-api dropped its bot
/// endpoint): DT/HR/EZ/HD combos first, then NM, then the DEFAULT catch-all.
pub fn candidate_chain(mods: &[String]) -> Vec<String> {
    let set: std::collections::HashSet<String> = mods.iter().map(|m| m.to_uppercase()).collect();
    let mut chain: Vec<String> = Vec::new();
    let mut found = false;

    if set.contains("DT") {
        if set.contains("HD") {
            chain.push("HDDT".into());
        }
        chain.push("DT".into());
        found = true;
    }
    if set.contains("HR") {
        if set.contains("HD") {
            chain.push("HDHR".into());
        }
        chain.push("HR".into());
        found = true;
    }
    if set.contains("EZ") {
        chain.push("EZ".into());
        found = true;
    }
    if set.contains("HD") {
        chain.push("HD".into());
        found = true;
    }
    if !found {
        chain.push("NM".into());
    }
    chain.push("DEFAULT".into());
    chain
}

/// Resolve a user's render pick for `mods` locally from their pick map.
pub async fn skin_pick(osu_id: i64, mods: &[String]) -> Result<Option<OscWebSkin>, Error> {
    let picks = get_user_picks(osu_id).await?;
    for modifier in candidate_chain(mods) {
        if let Some(Some(entry)) = picks.get(&modifier) {
            return Ok(Some(entry.to_skin(&modifier)));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, Deserialize)]
pub struct PickEntry {
    /// None for community picks.
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

    /// Absolute osk download URL on osc-web (ref omitted → the server resolves the
    /// owner's latest tag). `dir_name` is pushed as a path segment so spaces, `+`,
    /// and unicode are percent-encoded.
    fn osk_url(&self) -> String {
        let mut u = Url::parse(&base_url())
            .unwrap_or_else(|_| Url::parse("https://skins.sulej.net").unwrap());
        if let Ok(mut segs) = u.path_segments_mut() {
            segs.push("api");
            if self.is_community() {
                segs.push("community").push("osc");
            } else {
                segs.push("users")
                    .push(&self.owner_osu_id.unwrap_or(0).to_string());
            }
            segs.push("media").push("osk").push(&self.dir_name);
        }
        u.to_string()
    }

    fn to_skin(&self, matched_modifier: &str) -> OscWebSkin {
        OscWebSkin {
            dir_name: self.dir_name.clone(),
            url_path: self.osk_url(),
            // The pick map carries no skin.ini name; the dir_name is the label.
            skin_name: None,
            owner_osu_id: self.owner_osu_id,
            owner_kind: self.owner_kind.clone(),
            matched_modifier: Some(matched_modifier.to_string()),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn hd_dt_tries_hddt_then_dt_then_default() {
        assert_eq!(
            candidate_chain(&mods(&["HD", "DT"])),
            vec!["HDDT", "DT", "HD", "DEFAULT"]
        );
    }

    #[test]
    fn no_mods_falls_to_nm_then_default() {
        assert_eq!(candidate_chain(&[]), vec!["NM", "DEFAULT"]);
    }

    #[test]
    fn hr_hd_tries_hdhr_then_hr() {
        assert_eq!(
            candidate_chain(&mods(&["HR", "HD"])),
            vec!["HDHR", "HR", "HD", "DEFAULT"]
        );
    }
}
