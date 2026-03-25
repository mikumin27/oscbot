use rosu_v2::prelude as rosu;
use osu_db::Replay;
use std::env;
use std::path::PathBuf;
use rosu_pp::{Beatmap, Difficulty, Performance};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::osu::formatter::convert_osu_db_to_mod_array;
use crate::Error;

#[derive(Debug)]
pub struct CalculateScoreResponse {
    pub accuracy: f32,
    pub pp: f32,
    pub star_rating: f32,
}

// Helper function to get or download a beatmap file
async fn get_beatmap_file(map: &rosu::BeatmapExtended) -> Result<PathBuf, Error> {
    let cache_dir = env::var("OSC_BOT_DANSER_PATH")
        .map(|p| PathBuf::from(p).join("beatmap_cache"))
        .unwrap_or_else(|_| PathBuf::from("./beatmap_cache"));
    
    // Create cache directory if it doesn't exist
    fs::create_dir_all(&cache_dir).await.ok();
    
    let map_id = map.map_id;
    let beatmap_path = cache_dir.join(format!("{}.osu", map_id));
    
    // Return if file already exists
    if beatmap_path.exists() {
        return Ok(beatmap_path);
    }
    
    // Download from osu! API
    let url = format!("https://osu.ppy.sh/osu/{}", map_id);
    tracing::info!(map_id = map_id, "Downloading beatmap file for local PP calculation...");
    
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;
    
    // Save to cache
    let mut file = fs::File::create(&beatmap_path).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;
    
    tracing::info!(map_id = map_id, path = ?beatmap_path, "Beatmap file cached");
    Ok(beatmap_path)
}

// Convert mod array to GameMods bitmask
fn mods_to_bitmask(mods: &[String]) -> u32 {
    let mut bits = 0u32;
    for m in mods {
        bits |= match m.as_str() {
            "NF" => 1,
            "EZ" => 2,
            "TD" => 4,
            "HD" => 8,
            "HR" => 16,
            "SD" => 32,
            "DT" | "NC" => 64,
            "RX" => 128,
            "HT" => 256,
            "FL" => 1024,
            "SO" => 4096,
            _ => 0,
        };
    }
    bits
}

// Calculate accuracy from hit counts
fn calculate_accuracy(count_300: u32, count_100: u32, count_50: u32, count_miss: u32) -> f32 {
    let total_hits = count_300 + count_100 + count_50 + count_miss;
    if total_hits == 0 {
        return 100.0;
    }
    let weighted_hits = (count_300 * 300 + count_100 * 100 + count_50 * 50) as f32;
    let max_hits = (total_hits * 300) as f32;
    (weighted_hits / max_hits) * 100.0
}

pub async fn calculate_score_by_score(score: &rosu::Score) -> Result<CalculateScoreResponse, Error> {
    // Get beatmap from API
    let map = crate::osu::get_osu_instance()
        .beatmap()
        .map_id(score.map_id)
        .await?;
    
    let beatmap_path = get_beatmap_file(&map).await?;
    let beatmap = Beatmap::from_path(&beatmap_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse beatmap: {:?}", e))?;
    
    let mods: Vec<String> = score.mods.iter().map(|m| m.acronym().to_string()).collect();
    let mod_bits = mods_to_bitmask(&mods);
    
    // Calculate difficulty locally using rosu-pp
    let diff_attrs = Difficulty::new()
        .mods(mod_bits)
        .calculate(&beatmap);
    
    let stars = diff_attrs.stars();
    
    // Calculate performance locally using rosu-pp
    let perf_attrs = Performance::new(diff_attrs)
        .mods(mod_bits)
        .combo(score.max_combo)
        .n300(score.statistics.great)
        .n100(score.statistics.ok)
        .n50(score.statistics.meh)
        .misses(score.statistics.miss)
        .calculate();
    
    let pp = perf_attrs.pp() as f32;
    let accuracy = score.accuracy;
    
    Ok(CalculateScoreResponse {
        accuracy,
        pp,
        star_rating: stars as f32,
    })
}

pub async fn calculate_score_by_replay(replay: &Replay, map: &rosu::BeatmapExtended) -> Result<CalculateScoreResponse, Error> {
    let beatmap_path = get_beatmap_file(map).await?;
    let beatmap = Beatmap::from_path(&beatmap_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse beatmap: {:?}", e))?;
    
    let mods = convert_osu_db_to_mod_array(replay.mods);
    let mod_bits = mods_to_bitmask(&mods);
    
    // Calculate difficulty locally using rosu-pp
    let diff_attrs = Difficulty::new()
        .mods(mod_bits)
        .calculate(&beatmap);
    
    let stars = diff_attrs.stars();
    
    // Calculate performance locally using rosu-pp
    let perf_attrs = Performance::new(diff_attrs)
        .mods(mod_bits)
        .combo(replay.max_combo as u32)
        .n300(replay.count_300 as u32)
        .n100(replay.count_100 as u32)
        .n50(replay.count_50 as u32)
        .misses(replay.count_miss as u32)
        .calculate();
    
    let pp = perf_attrs.pp() as f32;
    let accuracy = calculate_accuracy(
        replay.count_300 as u32,
        replay.count_100 as u32,
        replay.count_50 as u32,
        replay.count_miss as u32,
    );
    
    Ok(CalculateScoreResponse {
        accuracy,
        pp,
        star_rating: stars as f32,
    })
}
