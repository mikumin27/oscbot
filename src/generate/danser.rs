use std::fs::{remove_dir_all, remove_file};
use std::io::Cursor;
use std::process::Stdio;
use std::env;
use std::path::Path;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::io::Write;
use tokio::process::Command;
use tokio::sync::mpsc;
use zip::ZipArchive;

use tokio::{fs::{File, create_dir_all}, io::AsyncWriteExt};
use tracing::Level;

use crate::apis::osc_web::{self, OscWebSkin};
use crate::discord_helper::ContextForFunctions;
use crate::{Error, db, embeds};
use crate::db::entities::user;

fn is_ffmpeg_progress_line(line: &str) -> bool {
    let l = line.trim_start();

    if l.starts_with("frame=") {
        return true;
    }

    l.contains("frame=") && l.contains("fps=") && l.contains("time=") && l.contains("speed=")
}

fn danser_stream_line_level(line: &str) -> Level {
    let l = line.trim();
    if l.is_empty() {
        return Level::TRACE;
    }

    if l.contains("Progress: ") {
        return Level::INFO;
    }

    if l.contains("danser-go version") {
        return Level::DEBUG;
    }
    if l.starts_with("ffmpeg version ") {
        return Level::DEBUG;
    }
    if l.starts_with("libav") {
        return Level::DEBUG;
    }

    if is_ffmpeg_progress_line(l) {
        if l.contains("q=-1.0") {
            return Level::DEBUG;
        }
        return Level::INFO;
    }

    if l.contains("video:0KiB") && l.contains("audio:") && l.contains("muxing overhead") {
        return Level::DEBUG;
    }

    if l.contains("video:") && l.contains("audio:") && l.contains("muxing overhead") {
        return Level::DEBUG;
    }
    if l.contains("Starting second pass: moving the moov atom") {
        return Level::DEBUG;
    }

    Level::TRACE
}

fn fallback_latest_rendered_video(output_dir: &str, started_at: SystemTime) -> Option<String> {
    let start_slack = started_at.checked_sub(Duration::from_secs(10)).unwrap_or(started_at);

    let mut best: Option<(SystemTime, u64, String)> = None;

    let entries = std::fs::read_dir(output_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("mp4")) != Some(true) {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.starts_with("danser_") {
            continue;
        }

        let meta = entry.metadata().ok()?;
        let modified = meta.modified().ok()?;
        if modified < start_slack {
            continue;
        }

        let size = meta.len();
        if size < 1024 * 1024 {
            // Avoid picking tiny/partial outputs.
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        match &best {
            None => best = Some((modified, size, path_str)),
            Some((best_modified, best_size, _)) => {
                if modified > *best_modified || (modified == *best_modified && size > *best_size) {
                    best = Some((modified, size, path_str));
                }
            }
        }
    }

    best.map(|(_, _, p)| p)
}

pub async fn render(cff: &ContextForFunctions<'_>, title: &String, beatmap_hash: &String, replay_reference: &String) -> Result<String, Error> {
    tracing::info!("Begin rendering replay");
    let started_at = SystemTime::now();
    let skin_path = &format!("{}/Skins/{}", env::var("OSC_BOT_DANSER_PATH").unwrap(), replay_reference);
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);

    let danser_cli = env::var("OSC_BOT_DANSER_CLI").unwrap_or("danser-cli".to_string());
    let stream_logs = env::var("OSC_BOT_DANSER_LOG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    let mut out = Command::new(&danser_cli);

    out.args(["-replay", replay_path, "-record"]);
    if Path::new(skin_path).is_dir() {
        out.args(["-skin", replay_reference]);
    }

    let mut danser_terminal = out
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = danser_terminal.stdout.take().unwrap();
    let stderr = danser_terminal.stderr.take().unwrap();

    let (tx, mut rx) = mpsc::unbounded_channel::<(String, String)>();

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(("stdout".to_string(), line));
            }
        });
    }

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(("stderr".to_string(), line));
            }
        });
    }

    drop(tx);

    let mut tail: VecDeque<String> = VecDeque::with_capacity(80);
    let mut video_path: Option<String> = None;
    let mut exit_status: Option<std::process::ExitStatus> = None;

    loop {
        tokio::select! {
            status = danser_terminal.wait() => {
                exit_status = Some(status?);
                break;
            }
            maybe = rx.recv() => {
                let Some((stream, line)) = maybe else {
                    break;
                };

                if stream_logs {
                    let level = danser_stream_line_level(&line);
                    match level {
                        Level::ERROR => tracing::error!("[danser {stream}] {line}"),
                        Level::WARN => tracing::warn!("[danser {stream}] {line}"),
                        Level::INFO => tracing::info!("[danser {stream}] {line}"),
                        Level::DEBUG => tracing::debug!("[danser {stream}] {line}"),
                        Level::TRACE => tracing::trace!("[danser {stream}] {line}"),
                    }
                    let _ = std::io::stdout().flush();
                    let _ = std::io::stderr().flush();
                }

                if tail.len() >= 80 {
                    tail.pop_front();
                }
                tail.push_back(format!("[{stream}] {line}"));

                if line.contains("Progress") {
                    if let Some((_, rest)) = line.split_once("Progress: ") {
                        cff.edit(
                            embeds::render_and_upload_embed(title, true, Some(rest.to_string()), false)?,
                            vec![]
                        ).await?;
                    }
                }

                if line.contains("Video is available at: ") {
                    if let Some((_, rest)) = line.split_once("Video is available at: ") {
                        video_path = Some(rest.replace("\\", "/").trim().to_string());
                        break;
                    }
                }
            }
        }
    }

    if let Some(path) = video_path {
        if exit_status.is_none() {
            let _ = danser_terminal.wait().await;
        }
        tracing::info!("Replay has been rendered successfully");
        return Ok(path);
    }

    let output_dir = format!("{}/videos", env::var("OSC_BOT_DANSER_PATH").unwrap());
    if exit_status.map(|s| s.success()).unwrap_or(false) {
        if let Some(path) = fallback_latest_rendered_video(&output_dir, started_at) {
            tracing::info!("Replay has been rendered successfully");
            return Ok(path);
        }
    }

    let failure = classify_danser_failure(&tail);
    let status_str = exit_status
        .map(|s| format!("{s}"))
        .unwrap_or_else(|| "unknown".to_string());
    tracing::error!(
        failure = ?failure,
        exit_status = %status_str,
        log_tail = ?tail,
        "danser failed to produce a video",
    );
    Err(failure.into())
}

#[derive(Debug, Clone)]
pub enum DanserFailure {
    BeatmapNotFound,
    NonStandardMode,
    ReplayInvalid,
    IncompatibleMods,
    FfmpegMissing,
    EncoderFailed,
    GraphicsInit,
    Crashed(String),
    NoOutput,
}

impl DanserFailure {
    pub fn user_message(&self) -> String {
        match self {
            Self::BeatmapNotFound => {
                "danser couldn't find this beatmap in osu!'s local database. \
                 The map may be unranked, loved, or otherwise unavailable for automated download."
                    .into()
            }
            Self::NonStandardMode => {
                "Only osu!standard replays can be rendered — taiko, ctb, and mania aren't supported.".into()
            }
            Self::ReplayInvalid => {
                "The replay file is invalid or missing input data — danser refused to load it.".into()
            }
            Self::IncompatibleMods => {
                "This replay uses an incompatible mod combination danser refuses to render.".into()
            }
            Self::FfmpegMissing => {
                "FFmpeg is missing or misconfigured inside the bot container. Ping the operator.".into()
            }
            Self::EncoderFailed => {
                "The video encoder died mid-render — the bot may be out of disk space. Ping the operator.".into()
            }
            Self::GraphicsInit => {
                "Graphics initialization failed inside the bot container. Ping the operator.".into()
            }
            Self::Crashed(detail) => {
                format!("danser crashed unexpectedly: `{}`. Ping the operator.", detail)
            }
            Self::NoOutput => {
                "danser exited without producing a video. The beatmap may be unavailable, \
                 or the replay file may be corrupted."
                    .into()
            }
        }
    }
}

impl std::fmt::Display for DanserFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_message())
    }
}

impl std::error::Error for DanserFailure {}

fn classify_danser_failure(tail: &VecDeque<String>) -> DanserFailure {
    for line in tail.iter().rev() {
        if line.contains("Beatmap not found") {
            return DanserFailure::BeatmapNotFound;
        }
        if line.contains("Modes other than osu!standard are not supported") {
            return DanserFailure::NonStandardMode;
        }
        if line.contains("Replay is missing input data") {
            return DanserFailure::ReplayInvalid;
        }
        if line.contains("Incompatible mods selected") {
            return DanserFailure::IncompatibleMods;
        }
        if line.contains("ffmpeg not found") || line.contains("ffmpeg was installed incorrectly") {
            return DanserFailure::FfmpegMissing;
        }
        if line.contains("ffmpeg finished abruptly") {
            return DanserFailure::EncoderFailed;
        }
        if line.contains("Failed to initialize GLFW") || line.contains("Failed to initialize OpenGL") {
            return DanserFailure::GraphicsInit;
        }
        if let Some((_, after)) = line.split_once("panic:") {
            let detail: String = after.trim().chars().take(160).collect();
            return DanserFailure::Crashed(detail);
        }
    }
    DanserFailure::NoOutput
}

pub async fn attach_replay(beatmap_hash: &String, replay_reference: &String, bytes: &Vec<u8>) -> Result<(), Error> {
    let replay_path = &format!("{}/Replays/{}", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH"), beatmap_hash);
    tracing::debug!(reference = replay_reference, path = replay_path, "Attaching replay...");
    if !Path::new(replay_path).is_dir() {
        // create_dir_all because /app/danser/Replays may not exist —
        // actions/upload-artifact drops empty directories during the
        // build, so the image ships without the parent.
        create_dir_all(&replay_path).await?;
    }

    let mut file = File::create(format!("{}/{}.osr", replay_path, replay_reference)).await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    tracing::debug!(path = format!("{}/{}.osr", replay_path, replay_reference), "replay attached");
    Ok(())
}

pub async fn get_replay(replay_reference: &String, beatmap_hash: &String) -> Result<osu_db::Replay, Error> {
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH"), beatmap_hash, replay_reference);
    tracing::debug!(path = replay_path, "Getting parsed replay...");
    let replay = osu_db::Replay::from_file(replay_path).unwrap();
    tracing::debug!(path = replay_path, "Replay found and returned");
    Ok(replay)
}

pub async fn get_replay_file(replay_reference: &String, beatmap_hash: &String) -> Result<File, Error> {
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);
    tracing::debug!(path = replay_path, "Getting replay as file...");

    let file = File::open(replay_path).await?;
    tracing::debug!(path = replay_path, "Replay found and returned");
    Ok(file)
}

pub async fn get_replay_bytes(replay_reference: &String, beatmap_hash: &String) -> Result<Vec<u8>, Error> {
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);
    tracing::debug!(path = replay_path, "Getting replay as bytes...");
    
    let bytes = fs::read(replay_path).await?;
    tracing::debug!(path = replay_path, "Replay found and returned");
    Ok(bytes)
}

pub async fn cleanup_files(beatmap_hash: &String, replay_reference: &String, video_path: &String) {
    tracing::debug!(reference = replay_reference, "Cleanup files for replay...");
    let beatmap_path = format!("{}/Songs/{}", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH must exist"), replay_reference);
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);
    let path = &format!("{}/Skins/{}", env::var("OSC_BOT_DANSER_PATH").unwrap(), replay_reference);
    remove_dir_all(path).ok();
    remove_dir_all(&beatmap_path).ok();
    remove_file(replay_path).ok();
    remove_file(video_path).ok();
}

pub async fn attach_skin_file(replay_reference: &String, url: &String) -> Result<bool, Error> {
    let path = &format!("{}/Skins/{}", env::var("OSC_BOT_DANSER_PATH").unwrap(), replay_reference);
    remove_dir_all(path).ok();
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?.error_for_status()?;

    let bytes = match resp.bytes().await {
        Ok(bytes) => bytes,
        Err(_) =>  return Ok(false)
    };

    tracing::debug!(url = url, "Skin has been downloaded successfully");
    
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)?;
    _ = zip.extract(path);
    tracing::debug!(url = url, path = path, "Skin has been extracted and saved");
    Ok(true)
}

pub async fn resolve_correct_skin(
    user: Option<user::Model>,
    identifier: Option<String>,
    mods: Vec<String>,
) -> Result<Option<OscWebSkin>, Error> {
    let user = match user {
        None => return Ok(None),
        Some(u) => u,
    };

    if let Some(id) = identifier {
        if let Some(legacy) = db::get_skin_by_identifier(user.clone(), id).await? {
            return Ok(Some(OscWebSkin {
                dir_name: legacy.identifier.clone(),
                url_path: legacy.url.clone(),
                matched_modifier: None,
            }));
        }
    }

    match osc_web::skin_pick(user.osu_id, &mods).await {
        Ok(Some(pick)) => {
            tracing::debug!(
                osu_id = user.osu_id,
                mods = ?mods,
                matched = ?pick.matched_modifier,
                dir_name = %pick.dir_name,
                "skin pick resolved via osc-web"
            );
            Ok(Some(pick))
        }
        Ok(None) => {
            tracing::debug!(osu_id = user.osu_id, mods = ?mods, "no skin pick in osc-web");
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(error = %e, "osc-web skin-pick failed; rendering without skin");
            Ok(None)
        }
    }
}