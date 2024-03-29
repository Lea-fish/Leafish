use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
    process::{exit, Command, Stdio},
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{TimeZone, Utc};
use serde::Deserialize;
use ureq::AgentBuilder;

const URL: &str = "https://api.github.com/repos/Lea-fish/Releases/releases/latest";
const CLIENT_JAR_PATH: &str = "./client.jar";
const ASSETS_FILE_NAME: &str = "assets.txt";
const ASSETS_META_PATH: &str = "./assets.txt";
const CLIENT_VER_PLACEHOLDER: &str = "%client_ver";

#[cfg(target_os = "windows")]
const MAIN_BINARY_PATH: &str = "./leafish.exe";
#[cfg(not(target_os = "windows"))]
const MAIN_BINARY_PATH: &str = "./leafish";

#[cfg(target_os = "windows")]
const BOOTSTRAP_BINARY_PATH: &str = "./bootstrap.exe";
#[cfg(not(target_os = "windows"))]
const BOOTSTRAP_BINARY_PATH: &str = "./bootstrap";

#[cfg(target_os = "windows")]
const UPDATED_BOOTSTRAP_BINARY_PATH: &str = "./bootstrap_new.exe";
#[cfg(not(target_os = "windows"))]
const UPDATED_BOOTSTRAP_BINARY_PATH: &str = "./bootstrap_new";

#[cfg(target_os = "windows")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "macos")]
const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.3; rv:123.0) Gecko/20100101 Firefox/123.0";
#[cfg(target_os = "linux")]
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux i686; rv:123.0) Gecko/20100101 Firefox/123.0";

fn main() {
    println!("[Info] Starting up bootstrap...");
    let args: Vec<String> = env::args().collect();
    let mut cmd = vec![];
    let mut provided_client = None;
    let mut no_update = false;
    for (idx, arg) in args.iter().enumerate() {
        // "noupdate" is required in order to support legacy installations
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
        if arg == "--client-jar" {
            provided_client = Some(args[idx + 1].clone());
        }
    }
    if no_update {
        // replace the client_ver placeholder with the actual version
        if let Some(provided_client) = provided_client.as_mut() {
            if let Ok(client_ver) = fs::read_to_string(ASSETS_META_PATH) {
                *provided_client = provided_client.replace(CLIENT_VER_PLACEHOLDER, &client_ver);
            }
        }
        // try to provide leafish with a client path
        if (provided_client.is_none()
            || !try_push_client_path(provided_client.as_ref().unwrap(), &mut cmd))
            && !try_push_client_path(CLIENT_JAR_PATH, &mut cmd)
        {
            println!("[Warn] (noupdate) Couldn't find client jar");
        }
        Command::new(
            fs::canonicalize(MAIN_BINARY_PATH)
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
        let _ = try_update(provided_client.as_ref());
        println!("[Info] Restarting bootstrap...");
        // shut down the process if we performed an update and let our parent bootstrap restart us, running a new version
        // otherwise we also have to shutdown in order not to restart leafish as soon as it is closed
        exit(0);
    }
}

fn try_push_client_path(provided_client: &str, cmd: &mut Vec<String>) -> bool {
    if Path::new(provided_client).exists() {
        match fs::canonicalize(provided_client) {
            Ok(path) => {
                if let Some(path) = path.to_str() {
                    cmd.push("--client-jar".to_string());
                    cmd.push(path.to_string());
                    return true;
                } else {
                    println!("[Warn] (noupdate) Couldn't convert client jar path to string");
                }
            }
            Err(err) => println!(
                "[Warn] (noupdate) Couldn't canonicalize client jar path: {}",
                err
            ),
        }
    }
    false
}

fn try_update(provided_client: Option<&String>) -> anyhow::Result<()> {
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
    let mut downloads: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
    for asset in latest.assets {
        #[allow(clippy::collapsible_if)]
        if asset.name == main_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_file(MAIN_BINARY_PATH),
            )? {
                println!("[Info] Downloading update for leafish binary...");
                downloads.push(thread::spawn(move || {
                    let mut new_binary = vec![];
                    ureq::get(&asset.browser_download_url)
                        .call()?
                        .into_reader()
                        .read_to_end(&mut new_binary)?;
                    let mut file = File::create(MAIN_BINARY_PATH)?;
                    file.write_all(&new_binary)?;
                    adjust_binary_perms(&file)?;
                    println!("[Info] Successfully updated leafish binary");
                    Ok(())
                }));
            }
        } else if asset.name == bootstrap_binary_name {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_file(BOOTSTRAP_BINARY_PATH),
            )? {
                println!("[Info] Downloading update for bootstrap...");
                downloads.push(thread::spawn(move || {
                    let mut new_binary = vec![];
                    ureq::get(&asset.browser_download_url)
                        .call()?
                        .into_reader()
                        .read_to_end(&mut new_binary)?;
                    fs::write(UPDATED_BOOTSTRAP_BINARY_PATH, &new_binary)?;
                    println!("[Info] Successfully downloaded bootstrap update");
                    Ok(())
                }));
            }
        } else if asset.name == ASSETS_FILE_NAME {
            if do_update(
                &latest.published_at,
                &asset.updated_at,
                time_stamp_file(ASSETS_META_PATH),
            )? || !Path::new(CLIENT_JAR_PATH).exists()
            {
                println!("[Info] Updating assets...");
                let provided_client = provided_client.cloned();
                downloads.push(thread::spawn(move || {
                    let raw_meta = ureq::get(&asset.browser_download_url)
                        .call()?
                        .into_string()?;

                    update_assets(&raw_meta, provided_client.as_ref())?;
                    println!("[Info] Updated assets");
                    Ok(())
                }));
            }
        }
    }
    let mut results = vec![];
    for download in downloads {
        let res = download.join();
        results.push(res);
    }
    for result in results {
        // FIXME: handle this more gracefully!
        let _ = result.unwrap();
    }
    Ok(())
}

fn update_assets(raw: &str, provided_client: Option<&String>) -> anyhow::Result<()> {
    let mut client_ver = None;
    let mut client_url = None;
    let lines = raw.split('\n');
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(": ") {
            let key = key.to_lowercase();
            match key.as_str() {
                "client" => {
                    client_url = Some(value.to_string());
                }
                "assets" => {
                    // FIXME: update assets description in the launcher
                    // FIXME: (this means that the client's asset expectations and the real asset version are out of sync)
                    // FIXME: following we have to support even outdated asset versions (to a certain degree)
                }
                "client-ver" => client_ver = Some(value.to_string()),
                _ => {}
            }
        } else {
            println!("[Warn] Couldn't read metadata line \"{line}\"");
        }
    }
    if let Some(client_url) = client_url {
        let update = if let Some((curr, provided_client)) = client_ver.as_ref().zip(provided_client)
        {
            !Path::new(&provided_client.replace(CLIENT_VER_PLACEHOLDER, curr)).exists()
        } else {
            true
        };
        if update {
            println!("[Info] Downloading client jar...");
            let mut client_jar = vec![];
            ureq::get(&client_url)
                .call()?
                .into_reader()
                .read_to_end(&mut client_jar)?;
            fs::write(CLIENT_JAR_PATH, &client_jar)?;
            println!("[Info] Downloaded client jar");
        }
    }
    // update the metadata as well
    let mut meta = File::create(ASSETS_META_PATH)?;
    if let Some(client_ver) = client_ver {
        meta.write_all(client_ver.as_bytes())?;
    } else {
        // FIXME: is this a good fallback?
        meta.set_modified(SystemTime::now())?;
    }
    Ok(())
}

fn time_stamp_file(path: &str) -> u64 {
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
fn adjust_binary_perms(file: &File) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o777);
    file.set_permissions(perms)?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn adjust_binary_perms(_file: &File) -> anyhow::Result<()> {
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
