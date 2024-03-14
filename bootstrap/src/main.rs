use std::{
    env, fs,
    path::Path,
    process::{exit, Command, Stdio},
    time::UNIX_EPOCH,
};

use chrono::{TimeZone, Utc};
use serde::Deserialize;
use ureq::AgentBuilder;

const URL: &str = "https://api.github.com/repos/Lea-fish/Releases/releases/latest";
const MAIN_BINARY_PATH: &str = "./leafish";
const UPDATED_BOOTSTRAP_BINARY_PATH: &str = "./bootstrap_new";
const BOOTSTRAP_BINARY_PATH: &str = "./bootstrap";
const CLIENT_JAR_PATH: &str = "./client.jar";
const ASSETS_FILE_NAME: &str = "assets.txt";

#[cfg(target_os = "windows")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "macos")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.3; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "linux")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux i686; rv:123.0) Gecko/20100101 Firefox/123.0";

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut cmd = vec![];
    let mut provided_client = false;
    let mut no_update = false;
    for (idx, arg) in args.iter().enumerate() {
        // "noupdate" is required in order to support legacy installations
        println!("got arg {arg}");
        if arg == "--noupdate" || arg == "noupdate" {
            no_update = true;
            continue;
        }
        if args.len() <= (idx + 1) {
            // skip multi parameter args in case the value is missing
            continue;
        }
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
        if arg == "--client-jar" && Path::new(&args[idx + 1]).exists() {
            match fs::canonicalize(&args[idx + 1]) {
                Ok(path) => {
                    if let Some(path) = path.to_str() {
                        cmd.push("--client-jar".to_string());
                        cmd.push(path.to_string());
                        provided_client = true;
                    } else {
                        println!("[Warn] Couldn't convert client jar path to string");
                    }
                }
                Err(err) => println!("[Warn] Couldn't canonicalize client jar path: {}", err),
            }
        }
    }
    if no_update {
        if !provided_client && Path::new(CLIENT_JAR_PATH).exists() {
            if let Ok(client_jar) = fs::canonicalize(CLIENT_JAR_PATH) {
                if let Some(client_jar) = client_jar.to_str() {
                    cmd.push("--client-jar".to_string());
                    cmd.push(client_jar.to_string());
                } else {
                    println!("[Warn] (noupdate) Couldn't convert client jar path to string");
                }
            } else {
                println!("[Warn] (noupdate) Couldn't canonicalize client jar path");
            }
        } else {
            println!("[Warn] (noupdate) Couldn't find client jar");
        }
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
        let _ = try_update(provided_client);
        println!("[Info] Restarting bootstrap...");
        // shut down the process if we performed an update and let our parent bootstrap restart us, running a new version
        // otherwise we also have to shutdown in order not to restart leafish as soon as it is closed
        exit(0);
    }
}

fn try_update(provided_client: bool) -> anyhow::Result<()> {
    println!("[Info] Checking for updates....");
    let latest = AgentBuilder::new()
        .user_agent(USER_AGENT)
        .build()
        .get(URL)
        .call()?
        .into_string()?;
    let latest: LatestResponse = serde_json::from_str(&latest).unwrap();
    println!("[Info] Looking for update files...");
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
        if asset.name == main_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_binary(MAIN_BINARY_PATH),
            )? {
                println!("[Info] Downloading update for leafish binary...");
                let mut new_binary = vec![];
                ureq::get(&asset.browser_download_url)
                    .call()?
                    .into_reader()
                    .read_to_end(&mut new_binary)?;
                fs::write(
                    format!("{}{}", MAIN_BINARY_PATH, env::consts::EXE_SUFFIX),
                    &new_binary,
                )?;
                adjust_binary_perms()?;
                println!("[Info] Successfully updated leafish binary");
            }
        } else if asset.name == bootstrap_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_binary(BOOTSTRAP_BINARY_PATH),
            )? {
                println!("[Info] Downloading update for bootstrap...");
                let mut new_binary = vec![];
                ureq::get(&asset.browser_download_url)
                    .call()?
                    .into_reader()
                    .read_to_end(&mut new_binary)?;
                fs::write(
                    format!(
                        "{}{}",
                        UPDATED_BOOTSTRAP_BINARY_PATH,
                        env::consts::EXE_SUFFIX
                    ),
                    &new_binary,
                )?;
                println!("[Info] Successfully downloaded bootstrap update");
            }
        } else if asset.name == ASSETS_FILE_NAME {
            load_links(&asset.name, provided_client)?;
        }
    }
    Ok(())
}

fn load_links(raw: &str, provided_client: bool) -> anyhow::Result<()> {
    let lines = raw.split('\n');
    for line in lines {
        if let Some((key, value)) = line.split_once(": ") {
            let key = key.to_lowercase();
            match key.as_str() {
                "client" => {
                    if provided_client || Path::new(CLIENT_JAR_PATH).exists() {
                        continue;
                    }
                    let mut client_jar = vec![];
                    ureq::get(value)
                        .call()?
                        .into_reader()
                        .read_to_end(&mut client_jar)?;
                    if let Err(err) = fs::write(CLIENT_JAR_PATH, &client_jar) {
                        println!("[Warn] error writing client jar {err}");
                    }
                }
                "assets" => {
                    // FIXME: update assets description in the launcher
                    // FIXME: (this means that the client's asset expectations and the real asset version are out of sync)
                    // FIXME: following we have to support even outdated asset versions (to a certain degree)
                }
                _ => {}
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
    let (date, time) = date[0..(date.len() - 2)].split_once('T').unwrap();
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
        .args(["777", file_name.as_path().to_str().unwrap()])
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
