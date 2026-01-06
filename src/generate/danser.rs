use std::fs::{remove_dir, remove_file};
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

use tokio::{fs::{File, create_dir}, io::AsyncWriteExt};

use crate::discord_helper::ContextForFunctions;
use crate::firebase::user;
use crate::{Error, embeds};

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

pub async fn render(cff: &ContextForFunctions<'_>, title: &String, beatmap_hash: &String, replay_reference: &String, user_id: &u32) -> Result<String, Error> {
    tracing::info!("Begin rendering replay");
    let started_at = SystemTime::now();
    let skin_path = &format!("{}/Skins/{}", env::var("OSC_BOT_DANSER_PATH").unwrap(), user_id);
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);

    let danser_cli = env::var("OSC_BOT_DANSER_CLI").unwrap_or("danser-cli".to_string());
    let stream_logs = env::var("OSC_BOT_DANSER_LOG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    let mut out = Command::new(&danser_cli);

    out.args(["-replay", replay_path, "-record"]);
    match user::get_user_skin(&user_id.to_string()).await {
        Some(url) => {
            if !Path::new(skin_path).is_dir() {
                attach_skin_file(*user_id, &url).await?;
            }
            out.args(["-skin", &user_id.to_string()]);
        },
        None => ()
    };

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
                    if stream == "stderr" {
                        tracing::info!("[danser {stream}] {line}");
                    } else {
                        tracing::debug!("[danser {stream}] {line}");
                    }
                    // Ensure logs show up promptly in non-tty Docker logging.
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
        // Ensure process is reaped; ignore errors.
        if exit_status.is_none() {
            let _ = danser_terminal.wait().await;
        }
        tracing::info!("Replay has been rendered successfully");
        return Ok(path);
    }

    // Newer danser builds don't always print "Video is available at:".
    // If rendering succeeded, fall back to picking the latest rendered mp4.
    let output_dir = format!("{}/videos", env::var("OSC_BOT_DANSER_PATH").unwrap());
    if exit_status.map(|s| s.success()).unwrap_or(false) {
        if let Some(path) = fallback_latest_rendered_video(&output_dir, started_at) {
            tracing::info!("Replay has been rendered successfully");
            return Ok(path);
        }
    }

    let status_str = exit_status
        .map(|s| format!("exit status: {s}"))
        .unwrap_or_else(|| "exit status: unknown".to_string());

    let mut msg = String::new();
    msg.push_str("Video could not be rendered (danser-cli did not report output path). ");
    msg.push_str(&status_str);
    msg.push_str(". Recent output:\n");
    for l in tail {
        msg.push_str(&l);
        msg.push('\n');
    }

    Err(msg.into())
}

pub async fn attach_replay(beatmap_hash: &String, replay_reference: &String, bytes: &Vec<u8>) -> Result<(), Error> {
    let replay_path = &format!("{}/Replays/{}", env::var("OSC_BOT_DANSER_PATH").expect("OSC_BOT_DANSER_PATH"), beatmap_hash);
    tracing::debug!(reference = replay_reference, path = replay_path, "Attaching replay...");
    if !Path::new(replay_path).is_dir() {
        create_dir(&replay_path).await?;
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
    let replay_path = &format!("{}/Replays/{}/{}.osr", env::var("OSC_BOT_DANSER_PATH").unwrap(), beatmap_hash, replay_reference);
    _ = remove_file(replay_path);
    _ = remove_file(video_path);
}

pub async fn attach_skin_file(user_id: u32, url: &String) -> Result<bool, Error> {
    tracing::debug!(user_id = user_id, url = url, "Save skin and tie to user...");
    let path = &format!("{}/Skins/{}", env::var("OSC_BOT_DANSER_PATH").unwrap(), user_id);
    _ = remove_dir(path);
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