use std::{collections::HashMap, fs::{self, File, OpenOptions}, io::{Read, Seek, Write}, path::Path};

use serde_derive::{Deserialize, Serialize};

// all of these are expected to be prepended with ".minecraft"
const DIR_PATH: &str = "/versions/Leafish/";
const DESC_JSON_PATH: &str = "/versions/Leafish/Leafish.json";
const JAR_PATH: &str = "/versions/Leafish/Leafish.jar";
const PROFILES_JSON_PATH: &str = "/launcher_profiles.json";
const LIBRARY_DIR_PATH: &str = "/libraries/leafish/Leafish/Jar/";
const LIBRARY_PATH: &str = "/libraries/leafish/Leafish/Jar/Leafish-Jar.jar";

// FIXME: add cli that allows reinstalling, uninstalling, installing and getting info

#[derive(Serialize, Deserialize)]
struct Description {
    id: String,
    #[serde(rename = "inheritsFrom")]
    inherits_from: String,
    time: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
    #[serde(rename = "type")]
    ty: String,
    libraries: Vec<Library>,
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: String,
}

#[derive(Serialize, Deserialize)]
struct Library {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct Profiles {
    profiles: HashMap<String, Profile>,
    settings: Settings,
    version: usize,
}

#[derive(Serialize, Deserialize)]
struct Profile {
    created: String,
    icon: String,
    #[serde(rename = "lastUsed")]
    last_used: String,
    #[serde(rename = "lastVersionId")]
    last_version_id: String,
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Serialize, Deserialize)]
struct Settings {
    #[serde(rename = "crashAssistance")]
    crash_assistance: bool,
    #[serde(rename = "enableAdvanced")]
    advanced: bool,
    #[serde(rename = "enableAnalytics")]
    analytics: bool,
    #[serde(rename = "enableHistorical")]
    historical: bool,
    #[serde(rename = "enableReleases")]
    releases: bool,
    #[serde(rename = "enableSnapshots")]
    snapshots: bool,
    #[serde(rename = "keepLauncherOpen")]
    keep_launcher_open: bool,
    #[serde(rename = "profileSorting")]
    profile_sorting: String,
    #[serde(rename = "showGameLog")]
    show_game_log: bool,
    #[serde(rename = "showMenu")]
    show_menu: bool,
    #[serde(rename = "soundOn")]
    sound_on: bool,
}

pub fn setup_launcher_wrapper(prefix: &str) -> anyhow::Result<bool> {
    let json_path = format!("{}{}", prefix, DESC_JSON_PATH);
    let jar_path = format!("{}{}", prefix, JAR_PATH);
    if Path::new(&json_path).exists() && Path::new(&jar_path).exists() {
        println!("Leafish is already installed");
        return Ok(false);
    }
    // cleanup old files
    if Path::new(&json_path).exists() {
        println!("Removing old json...");
        fs::remove_file(&json_path)?;
    }
    if Path::new(&jar_path).exists() {
        println!("Removing old jar...");
        fs::remove_file(&jar_path)?;
    }

    println!("Removing old json...");
    let dir_path = format!("{}{}", prefix, DIR_PATH);
    if !Path::new(&dir_path).exists() {
        println!("Creating version directory...");
        // create directory if necessary
        fs::create_dir(&dir_path)?;
    }

    // write files
    println!("Creating json...");
    let mut json = File::create_new(&json_path)?;
    json.write_all(serde_json::to_string_pretty(&Description {
        id: "Leafish".to_string(),
        inherits_from: "1.8.9".to_string(), // FIXME: is this a good default?
        time: "2020-01-01T00:00:00+02:00".to_string(), // FIXME: use some sensible time
        release_time: "2020-01-01T00:00:00+02:00".to_string(),
        ty: "release".to_string(),
        libraries: vec![Library { name: "leafish:Leafish:Jar".to_string() }], // we need nobody, but ourselves ;)
        main_class: "de.leafish.Main".to_string(),
        minecraft_arguments: "--username ${auth_player_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type}".to_string(),
    })?.as_bytes())?;

    // FIXME: download the latest jar from github
    let raw_jar = include_bytes!("../resources/wrapper.jar");

    println!("Copying version jar...");
    let mut jar = File::create_new(&jar_path)?;
    jar.write_all(raw_jar)?;

    let lib_dir = format!("{}{}", prefix, LIBRARY_DIR_PATH);
    if !Path::new(&lib_dir).exists() {
        println!("Copying library...");
        fs::create_dir_all(&lib_dir)?;
        let mut file = File::create_new(format!("{}{}", prefix, LIBRARY_PATH))?;
        file.write_all(raw_jar)?;
    }

    println!("Installation successful!");

    /*let profiles_path = format!("{}{}", prefix, PROFILES_JSON_PATH);
    if !Path::new(&profiles_path).exists() {
        // FIXME: error
    }
    let mut profiles_json_file = OpenOptions::new().append(false).write(true).read(true).open(&profiles_path)?;
    let mut profiles = String::new();
    profiles_json_file.read_to_string(&mut profiles)?;
    let mut profiles: Profiles = serde_json::from_str(&profiles)?;
    let now = Utc::now();
    let now = now.format("%Y-%m-%dT%H:%M:%S.000Z");
    
    profiles.profiles.insert("Leafish".to_string(), Profile {
        created: now.to_string(),
        icon: "Furnace".to_string(), // TODO: look for a cool block we could use ;)
        last_used: now.to_string(),
        last_version_id: "Leafish".to_string(), // FIXME: choose a better default
        name: "Leafish".to_string(),
        ty: "custom".to_string(),
    });

    profiles_json_file.set_len(0)?;
    profiles_json_file.seek(std::io::SeekFrom::Start(0))?;
    profiles_json_file.write_all(serde_json::to_string_pretty(&profiles)?.as_bytes())?;*/

    Ok(true)
}
