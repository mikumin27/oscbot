use rosu_v2::prelude as rosu;

use crate::osu::{self, formatter::mods_string};

pub async fn generate_title_with_score(score: &rosu::Score, map: &rosu::BeatmapExtended) -> String {
    tracing::info!("Generating title by score...");
    let username: &String = &score.user.as_ref().expect("User must exist").username.to_string();
    let mods = mods_string(&score.mods);
    
    // Calculate star rating using rosu-pp
    let stars = match osu::pp_calculator::calculate_score_by_score(score).await {
        Ok(result) => result.star_rating,
        Err(_) => 0.0,
    };
    
    generate_title(map, &username, stars, mods)
}

pub async fn generate_title_with_replay(replay: &osu_db::Replay, map: &rosu::BeatmapExtended) -> String {
    tracing::info!("Generating title by replay...");
    let mods = osu::formatter::convert_osu_db_to_mod_array(replay.mods);
    
    // Calculate star rating using rosu-pp
    let stars = match osu::pp_calculator::calculate_score_by_replay(replay, map).await {
        Ok(result) => result.star_rating,
        Err(_) => 0.0,
    };
    
    generate_title(map, replay.player_name.as_ref().unwrap_or(&"Unknown player".to_string()), stars, mods.join(""))
}

fn generate_title(map: &rosu::BeatmapExtended, username: &String, stars: f32, mods: String) -> String {
    let mapset = map.mapset.as_ref().expect("missing mapset");

    if mods != "" {
         return format!("{} | {} - {} [{}] {:.2}⭐ +{}", username, mapset.artist, mapset.title, map.version, stars, mods);
    }

    let title = format!("{} | {} - {} [{}] {:.2}⭐", username, mapset.artist, mapset.title, map.version, stars);
    tracing::info!("Title has been generated successfully");
    title
}

pub fn generate_description(userid: u32, mapid: u32, score: Option<&rosu::Score>, time_string: Option<String>) -> String {
    tracing::info!("Generating description");
    let fmt = time::format_description::parse("[day].[month].[year] at [hour]:[minute]").unwrap();
    let timestamp = match time_string {
        Some(timestamp) => timestamp,
        None => score.expect("Score must exist").ended_at.format(&fmt).unwrap()
    };

    let score_link = match score {
        Some(score) => format!("Score: https://osu.ppy.sh/scores/{}", score.id),
        None => "\nScore was rendered by a replay file".to_string(),
    };

    let description = format!("
This score was set on {}.

Player: https://osu.ppy.sh/users/{}
Beatmap: https://osu.ppy.sh/beatmaps/{}
{}

Join the osu swiss community in discord: https://discord.com/invite/SHz8QtD", timestamp, userid, mapid, score_link);
    tracing::info!("Description has been generated successfully");
    description
}