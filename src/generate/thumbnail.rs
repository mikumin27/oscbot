use ab_glyph::{FontRef, PxScale};
use image::{DynamicImage, GenericImage, Rgba, imageops::FilterType};
use imageproc::drawing::draw_text_mut;
use osu_db::Replay;
use rosu_v2::prelude::{self as rosu};
use std::io::Cursor;
use crate::{generate::image_binaries, osu};

const SPACE_BETWEEN_MODS: u32 = 20;

const MOD_WIDTH: u32 = 100;

async fn open_image_from_url(url: &str) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let bytes = reqwest::get(url).await.expect("Thumbnail to exist").bytes().await?;
    Ok(image::load_from_memory(&bytes)?)
}

fn blur_section(img: &mut DynamicImage, x: u32, y: u32, w: u32, h: u32, sigma: f32) {
    let mut rgba = img.to_rgba8();

    let sub = image::imageops::crop_imm(&rgba, x, y, w, h).to_image();
    let mut sub = DynamicImage::ImageRgba8(sub);
    sub = sub.fast_blur(sigma);

    rgba.copy_from(&sub.to_rgba8(), x, y).unwrap();
    *img = DynamicImage::ImageRgba8(rgba);
}


fn dim(img: &mut DynamicImage, factor: f32) {
    let mut rgba = img.to_rgba8();

    for p in rgba.pixels_mut() {
        p.0[0] = (p.0[0] as f32 * factor) as u8;
        p.0[1] = (p.0[1] as f32 * factor) as u8;
        p.0[2] = (p.0[2] as f32 * factor) as u8;
    }

    *img = DynamicImage::ImageRgba8(rgba);
}

fn round_corners(img: &mut DynamicImage, r: u32) {
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let r2 = (r * r) as i32;

    for y in 0..h {
        for x in 0..w {
            let cx = if x < r { r - x - 1 } else if x >= w - r { x - (w - r) } else { continue };
            let cy = if y < r { r - y - 1 } else if y >= h - r { y - (h - r) } else { continue };

            if (cx * cx + cy * cy) as i32 > r2 {
                rgba.get_pixel_mut(x, y).0[3] = 0;
            }
        }
    }

    *img = DynamicImage::ImageRgba8(rgba);
}

fn write_centered(img: &mut DynamicImage, color: &Rgba<u8>, cx: i32, cy: i32, scale: PxScale, font: &FontRef, text: &str) {
    let (w, h) = imageproc::drawing::text_size(scale, font, text);

    let x = cx - (w / 2) as i32;
    let y = cy - (h / 2) as i32;

    draw_text_mut(img, *color, x, y, scale, &font, text);
}

pub async fn generate_thumbnail_from_replay_file(replay: &Replay, map: &rosu::BeatmapExtended, subtitle: &str) -> Vec<u8> {
    tracing::info!(replay_hash = replay.replay_hash, "Generating thumbnail from replay file...");
    let user = osu::get_osu_instance().user(replay.player_name.as_ref().expect("Expect a username")).await.expect("Player to exist");
    let result = osu::pp_calculator::calculate_score_by_replay(replay, &map).await.expect("Local PP calculation to succeed");
    let mods = osu::formatter::convert_osu_db_to_mod_array(replay.mods);
    let grade = osu::formatter::calculate_grade_from_accuracy(result.accuracy, replay.count_miss > 0, mods.contains(&"HD".to_string()));
    generate_thumbnail(user, map, subtitle, Some(result.pp), result.accuracy, replay.max_combo as u32, mods, &grade).await
}

pub async fn generate_thumbnail_from_score(score: &rosu::Score, map: &rosu::BeatmapExtended, subtitle: &str) -> Vec<u8> {
    tracing::info!(scoreid = score.id, "Generating thumbnail from score...");
    let user = score.get_user(osu::get_osu_instance()).await.expect("User should exist");
    let mods: Vec<String> = score.mods.iter().map(|beatmap| beatmap.acronym().to_string()).collect();
    generate_thumbnail(user, map, subtitle, score.pp, score.accuracy, score.max_combo, mods, &score.grade).await
}

async fn generate_thumbnail(user: rosu::UserExtended, map: &rosu::BeatmapExtended, subtitle: &str, pp: Option<f32>, accuracy: f32, max_combo: u32, mods: Vec<String>, grade:&rosu::Grade) -> Vec<u8> {
    let user_stats = user.statistics.as_ref().expect("Stats must exist");
    let mapset = map.mapset.as_ref().expect("Mapset must exist");

    let global_rank: &str = match user_stats.global_rank {
        Some(rank) => &format!("#{}", rank),
        None => "Unranked",
    };

    let country_rank: &str = match user_stats.country_rank {
        Some(rank) => &format!("#{}", rank),
        None => "Unranked",
    };

    let pp: &str = match pp {
        Some(pp) => &format!("{}pp", pp as u32),
        None => "Unranked",
    };

    let mut buf = Vec::new();

    let mut score_bg = open_image_from_url(&mapset.covers.cover_2x).await.unwrap_or_else(|_| {
        image::load_from_memory(image_binaries::DEFAULT_BACKGROUND).unwrap()
    });
    score_bg = score_bg.resize_to_fill(1920, 1080, FilterType::Nearest);
    dim(&mut score_bg, 0.7);
    blur_section(&mut score_bg, 0, 310, 1920, 770, 10.0);
    
    let thumbnail_template = image::load_from_memory(image_binaries::TEMPLATE_MAIN).unwrap();
    image::imageops::overlay(&mut score_bg, &thumbnail_template, 0, 0);

    let mut user_profile_picture = open_image_from_url(&user.avatar_url).await.expect("Profile picture must exist");
    user_profile_picture = user_profile_picture.resize(300, 300, FilterType::Nearest);
    let _ = round_corners(&mut user_profile_picture, 85);
    image::imageops::overlay(&mut score_bg, &user_profile_picture, 812, 443);

    let white = Rgba([255, 255, 255, 255]);
    let font = FontRef::try_from_slice(image_binaries::FONT_ALLER_BD.iter().as_slice()).unwrap();

    write_centered(&mut score_bg, &white, 960, 300, PxScale::from(70.0), &font, &user.username);
    write_centered(&mut score_bg, &white, 1550, 300, PxScale::from(80.0), &font, &format!("{:.2}%", accuracy));
    write_centered(&mut score_bg, &white, 375, 300, PxScale::from(80.0), &font, &format!("{}x", max_combo));
    draw_text_mut(&mut score_bg, white, 1305, 570, PxScale::from(70.0), &font, global_rank);
    draw_text_mut(&mut score_bg, white, 1305, 662, PxScale::from(70.0), &font, country_rank);
    write_centered(&mut score_bg, &Rgba([222, 222, 222, 255]), 1360, 460, PxScale::from(80.0), &font, pp);

    write_centered(&mut score_bg, &Rgba([222, 222, 222, 255]), 960, 60, PxScale::from(80.0), &font, &mapset.artist);
    write_centered(&mut score_bg, &white, 960, 140, PxScale::from(90.0), &font, &mapset.title);
    write_centered(&mut score_bg, &white, 960, 950, PxScale::from(60.0), &font, subtitle);

    let length = mods.len();
    let start = 960 - (length as u32 * (MOD_WIDTH + SPACE_BETWEEN_MODS) / 2);
    for (i, gamemod) in mods.iter().enumerate() {
        let current_round: u32 = i as u32;
        let mod_image = image::load_from_memory(image_binaries::get_mod_bytes(gamemod)).unwrap();
        image::imageops::overlay(&mut score_bg, &mod_image, (start + (current_round * (MOD_WIDTH + SPACE_BETWEEN_MODS))) as i64, 770);
    }

    let mut grade_image = image::load_from_memory(image_binaries::get_rank_bytes(grade)).unwrap();
    grade_image = grade_image.resize((grade_image.width() as f32 / 2.5) as u32, (grade_image.height() as f32 / 2.5) as u32, FilterType::Nearest);
    image::imageops::overlay(&mut score_bg, &grade_image, 490, 450);
    
    let _ = score_bg.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png);
    tracing::info!("Thumbnail has been generated");
    buf
}