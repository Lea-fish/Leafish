use std::{
    env, fs,
    process::{exit, Command, Stdio},
    time::UNIX_EPOCH,
};

use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde_derive::Deserialize;

const URL: &str = "https://api.github.com/repos/Lea-fish/Releases/releases/latest";
const MAIN_BINARY_PATH: &str = "./leafish";
const UPDATED_BOOTSTRAP_BINARY_PATH: &str = "./bootstrap_new";
const BOOTSTRAP_BINARY_PATH: &str = "./bootstrap";

#[cfg(target_os = "windows")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "macos")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.3; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "linux")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux i686; rv:123.0) Gecko/20100101 Firefox/123.0";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut cmd = vec![];
    for (idx, arg) in args.iter().enumerate() {
        if arg == "--uuid" {
            cmd.push("--uuid".to_string());
            cmd.push(args[idx + 1].clone());
            continue;
        }
        if arg == "--username" {
            cmd.push("--name".to_string());
            cmd.push(args[idx + 1].clone());
            continue;
        }
        if arg == "--accessToken" {
            cmd.push("--token".to_string());
            cmd.push(args[idx + 1].clone());
            continue;
        }
        if arg == "--assetIndex" {
            cmd.push("--asset-index".to_string());
            cmd.push(args[idx + 1].clone());
        }
        if arg == "--assetsDir" {
            cmd.push("--assets-dir".to_string());
            cmd.push(args[idx + 1].clone());
        }
    }
    if args[args.len() - 1] == "noupdate" {
        Command::new(
            fs::canonicalize(format!("{}{}", MAIN_BINARY_PATH, env::consts::EXE_SUFFIX))
                .unwrap()
                .as_path()
                .to_str()
                .unwrap(),
        )
        .args(cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    } else {
        let _ = try_update().await;
        println!("[INFO] Restarting bootstrap...");
        // shut down the process if we performed an update and let our parent bootstrap restart us, running a new version
        // otherwise we also have to shutdown in order not to restart leafish as soon as it is closed
        exit(0);
    }
}

async fn try_update() -> anyhow::Result<()> {
    println!("[INFO] Checking for updates....");
    let latest = Client::builder()
        .user_agent(USER_AGENT)
        .build()?
        .get(URL)
        .send()
        .await?
        .text()
        .await?;
    let latest: LatestResponse = serde_json::from_str(&latest).unwrap();
    println!("[INFO] Looking for update files...");
    let bootstrap_binary_name = format!(
        "bootstrap_{}_{}{}",
        env::consts::ARCH,
        env::consts::OS,
        env::consts::EXE_SUFFIX
    );
    let main_binary_name = format!(
        "leafish_{}_{}{}",
        env::consts::ARCH,
        env::consts::OS,
        env::consts::EXE_SUFFIX
    );
    for asset in latest.assets {
        if &asset.name == &main_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_binary(MAIN_BINARY_PATH),
            )? {
                println!("[INFO] Downloading update for leafish binary...");
                let new_binary = reqwest::get(&asset.browser_download_url)
                    .await?
                    .bytes()
                    .await?;
                fs::write(
                    format!("{}{}", MAIN_BINARY_PATH, env::consts::EXE_SUFFIX),
                    &new_binary,
                )?;
                adjust_binary_perms()?;
                println!("[INFO] Successfully updated leafish binary");
            }
        } else if &asset.name == &bootstrap_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_binary(BOOTSTRAP_BINARY_PATH),
            )? {
                println!("[INFO] Downloading update for bootstrap...");
                let new_binary = reqwest::get(&asset.browser_download_url)
                    .await?
                    .bytes()
                    .await?;
                fs::write(
                    format!(
                        "{}{}",
                        UPDATED_BOOTSTRAP_BINARY_PATH,
                        env::consts::EXE_SUFFIX
                    ),
                    &new_binary,
                )?;
                println!("[INFO] Successfully downloaded bootstrap update");
            }
        }
    }
    Ok(())
}

fn time_stamp_binary(path: &str) -> u64 {
    fs::metadata(path)
        .map(|meta| {
            let modified = meta
                .modified()
                .map(|time| {
                    time.duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as u64
                })
                .unwrap_or(0);
            let created = meta
                .created()
                .map(|time| {
                    time.duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as u64
                })
                .unwrap_or(0);
            modified.max(created)
        })
        .unwrap_or(0)
}

fn parse_date(date: &str) -> anyhow::Result<i64> {
    // FIXME: get rid of all the unwraps
    let (date, time) = (&date[0..(date.len() - 2)]).split_once('T').unwrap();
    let mut date_parts = date.split('-');
    let years = date_parts.next().unwrap();
    let months = date_parts.next().unwrap();
    let days = date_parts.next().unwrap();
    let mut time_parts = time.split(':');
    let hours = time_parts.next().unwrap();
    let minutes = time_parts.next().unwrap();
    let seconds = time_parts.next().unwrap();
    let time = Utc
        .with_ymd_and_hms(
            years.parse::<u32>()? as i32,
            months.parse::<u32>()?,
            days.parse::<u32>()?,
            hours.parse::<u32>()?,
            minutes.parse::<u32>()?,
            seconds.parse::<u32>()?,
        )
        .unwrap();
    let date_millis = time.timestamp_millis();
    Ok(date_millis)
}

fn do_update(
    published_date: &str,
    curr_updated_date: &str,
    last_modified: u64,
) -> anyhow::Result<bool> {
    let published_millis = parse_date(published_date)?;
    let curr_millis = parse_date(curr_updated_date)?;
    if published_millis <= 0 || curr_millis <= 0 {
        return Ok(false);
    }
    let curr_millis = curr_millis.max(published_millis);
    Ok(curr_millis as u64 > last_modified)
}

// FIXME: should we do this for mac as well?
#[cfg(target_os = "linux")]
fn adjust_binary_perms() -> anyhow::Result<()> {
    let file_name =
        fs::canonicalize(format!("{}{}", MAIN_BINARY_PATH, env::consts::EXE_SUFFIX)).unwrap();
    Command::new("chmod")
        .args(&["777", file_name.as_path().to_str().unwrap()])
        .spawn()?
        .wait()?; // FIXME: check for exit status!
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn adjust_binary_perms() -> anyhow::Result<()> {
    Ok(())
}

#[derive(Deserialize)]
struct LatestResponse {
    assets: Vec<Asset>,
    published_at: String,
}

#[derive(Deserialize)]
struct Asset {
    browser_download_url: String,
    name: String,
    updated_at: String,
}
