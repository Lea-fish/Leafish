use std::{env, fs, process::{exit, Command, Stdio}, time::UNIX_EPOCH};

use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde_derive::Deserialize;

const URL: &str = "https://api.github.com/repos/Lea-fish/Releases/releases/latest";
const MAIN_BINARY_NAME: &str = "leafish";
const BOOTSTRAP_BINARY_NAME: &str = "bootstrap";
const MAIN_BINARY_PATH: &str = "./leafish";
const BOOTSTRAP_BINARY_PATH: &str = "./bootstrap_new";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut cmd = vec!["--uuid".to_string(), String::new(), "--name".to_string(), String::new(), "--token".to_string(), String::new()];
    for (idx, arg) in args.iter().enumerate() {
        if arg == "--uuid" {
            cmd[1] = args[idx + 1].clone();
            continue;
        }
        if arg == "--username" {
            cmd[3] = args[idx + 1].clone();
            continue;
        }
        if arg == "--token" {
            cmd[5] = args[idx + 1].clone();
            continue;
        }
    }
    let _ = try_update().await;

    Command::new(MAIN_BINARY_PATH).args(cmd).stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn().unwrap();
}

async fn try_update() -> anyhow::Result<()> {
    println!("start try update!");
    let latest = Client::builder().user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.1").build()?.get(URL).send().await?.text().await?;
    println!("got response {latest}");
    let latest: LatestResponse = serde_json::from_str(&latest).unwrap();
    println!("acquired info");
    let mut performed_update = false;
    let (update_main, update_bootstrap) = do_update(&latest.published_at, time_stamp_binary(MAIN_BINARY_PATH), time_stamp_binary(BOOTSTRAP_BINARY_PATH))?;
    println!("main: {} bootstrap: {}", time_stamp_binary(MAIN_BINARY_PATH), time_stamp_binary(BOOTSTRAP_BINARY_PATH));
    if update_main || update_bootstrap {
        println!("try update!");
        for asset in latest.assets {
            if &asset.name == MAIN_BINARY_NAME && update_main {
                let new_binary = reqwest::get(&asset.browser_download_url).await?.bytes().await?;
                fs::write(MAIN_BINARY_PATH, &new_binary)?;
                adjust_binary_perms()?;
            } else if &asset.name == BOOTSTRAP_BINARY_NAME && update_bootstrap {
                let new_binary = reqwest::get(&asset.browser_download_url).await?.bytes().await?;
                fs::write(BOOTSTRAP_BINARY_PATH, &new_binary)?;
                performed_update = true;
            }
        }
    }
    if performed_update {
        // shut down the process if we performed an update and let our parent bootstrap restart us, running a new version
        exit(0);
    }
    Ok(())
}

fn time_stamp_binary(path: &str) -> u64 {
    fs::metadata(path).map(|meta| {
        let modified = meta.modified().map(|time| time.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64).unwrap_or(0);
        let created = meta.created().map(|time| time.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64).unwrap_or(0);
        modified.max(created)
    }).unwrap_or(0)
}

fn do_update(published_date: &str, last_modified_main: u64, last_modified_bootstrap: u64) -> anyhow::Result<(bool, bool)> {
    // FIXME: get rid of all the unwraps
    let (date, time) = (&published_date[0..(published_date.len() - 2)]).split_once('T').unwrap();
    let mut date_parts = date.split('-');
    let years = date_parts.next().unwrap();
    let months = date_parts.next().unwrap();
    let days = date_parts.next().unwrap();
    let mut time_parts = time.split(':');
    let hours = time_parts.next().unwrap();
    let minutes = time_parts.next().unwrap();
    let seconds = time_parts.next().unwrap();
    let published = Utc.with_ymd_and_hms(years.parse::<u32>()? as i32, months.parse::<u32>()?,
    days.parse::<u32>()?, hours.parse::<u32>()?, minutes.parse::<u32>()?, seconds.parse::<u32>()?).unwrap();
    let date_millis = published.timestamp_millis();
    if date_millis <= 0 {
        return Ok((false, false));
    }
    Ok(((date_millis as u64 > last_modified_main), (date_millis as u64 > last_modified_bootstrap)))
}

// FIXME: should we do this for mac as well?
#[cfg(target_os = "linux")]
fn adjust_binary_perms() -> anyhow::Result<()> {
    Command::new("chmod").args(&["777", fs::canonicalize(MAIN_BINARY_PATH)?.to_str().unwrap()]).spawn()?.wait()?; // FIXME: check for exit status!
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn adjust_bootstrap_perms() -> anyhow::Result<()> {
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
}