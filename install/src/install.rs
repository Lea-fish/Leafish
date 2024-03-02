use std::{fs, path::Path};

pub mod mojang {

    use std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::{Read, Seek, Write},
        path::Path,
    };

    use chrono::Utc;
    use serde_derive::{Deserialize, Serialize};
    use serde_with::skip_serializing_none;

    use crate::install::mk_dir;

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

    #[skip_serializing_none]
    #[derive(Serialize, Deserialize)]
    struct Profile {
        created: Option<String>,
        icon: String,
        #[serde(rename = "lastUsed")]
        last_used: String,
        #[serde(rename = "lastVersionId")]
        last_version_id: Option<String>,
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

    pub fn setup(prefix: &str) -> anyhow::Result<bool> {
        if !Path::new(prefix).exists() {
            println!("[Info] Couldn't find .minecraft directory");
            return Ok(false);
        }

        let json_path = format!("{}{}", prefix, DESC_JSON_PATH);
        let jar_path = format!("{}{}", prefix, JAR_PATH);
        if Path::new(&json_path).exists() && Path::new(&jar_path).exists() {
            println!("[Info] Leafish is already installed");
            return Ok(false);
        }
        // cleanup old files
        if Path::new(&json_path).exists() {
            println!("[Info] Removing old json...");
            fs::remove_file(&json_path)?;
        }
        if Path::new(&jar_path).exists() {
            println!("[Info] Removing old jar...");
            fs::remove_file(&jar_path)?;
        }

        let dir_path = format!("{}{}", prefix, DIR_PATH);
        // create version directory if necessary
        mk_dir(&dir_path, "[Info] Creating version directory...")?;

        // write files
        println!("[Info] Creating json...");
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

        println!("[Info] Copying version jar...");
        let mut jar = File::create_new(&jar_path)?;
        jar.write_all(raw_jar)?;

        let lib_dir = format!("{}{}", prefix, LIBRARY_DIR_PATH);
        if !Path::new(&lib_dir).exists() {
            println!("[Info] Copying library...");
            fs::create_dir_all(&lib_dir)?;
            let mut file = File::create_new(format!("{}{}", prefix, LIBRARY_PATH))?;
            file.write_all(raw_jar)?;
        }

        let profiles_path = format!("{}{}", prefix, PROFILES_JSON_PATH);
        if !Path::new(&profiles_path).exists() {
            println!(
                "[Warn] Couldn't create profile as the file {} doesn't exist",
                profiles_path
            );
            return Ok(true);
        }
        println!("[Info] Installing profile...");
        let mut profiles_json_file = OpenOptions::new()
            .append(false)
            .write(true)
            .read(true)
            .open(&profiles_path)?;
        let mut profiles = String::new();
        profiles_json_file.read_to_string(&mut profiles)?;
        let mut profiles: Profiles = serde_json::from_str(&profiles)?;
        let now = Utc::now();
        let now = now.format("%Y-%m-%dT%H:%M:%S.000Z");

        profiles.profiles.insert(
            "Leafish".to_string(),
            Profile {
                created: Some(now.to_string()),
                icon: include_str!("../resources/icon.txt").to_string(),
                last_used: now.to_string(),
                last_version_id: Some("Leafish".to_string()),
                name: "Leafish".to_string(),
                ty: "custom".to_string(),
            },
        );

        profiles_json_file.set_len(0)?;
        profiles_json_file.seek(std::io::SeekFrom::Start(0))?;
        profiles_json_file.write_all(serde_json::to_string_pretty(&profiles)?.as_bytes())?;

        println!("[Info] Installation into .minecraft directory successful");

        Ok(true)
    }
}

pub mod prism {
    use std::{fs, path::Path};

    use serde_derive::Serialize;
    use serde_with::skip_serializing_none;

    use crate::install::mk_dir;

    const ICONS_DIR_PATH: &str = "/icons";
    const ICON_PATH: &str = "/icons/leafish";
    const INSTANCE_DIR_PATH: &str = "/instances/Leafish";
    const CFG_PATH: &str = "/instances/Leafish/instance.cfg";
    const PACK_PATH: &str = "/instances/Leafish/mmc-pack.json";
    const META_DIR_PATH: &str = "/meta/net.minecraft";
    const META_PATH: &str = "/meta/net.minecraft/Leafish.json";
    const LIB_DIR_PATH: &str = "/libraries/de/leafish/";

    const DEFAULT_CFG: &str = "[General]
    ConfigVersion=1.2
    iconKey=leafish
    name=Leafish
    InstanceType=OneSix";

    pub fn setup(prefix: &str) -> anyhow::Result<bool> {
        if !Path::new(prefix).exists() {
            // FIXME: this will print twice, fix this
            println!("[Info] [PrismLauncher] Couldn't find PrismLauncher directory");
            return Ok(false);
        }

        let dir_path = format!("{}{}", prefix, INSTANCE_DIR_PATH);
        if Path::new(&dir_path).exists() {
            println!("[Info] [PrismLauncher] Profile already exists");
            return Ok(false);
        }
        mk_dir(
            &dir_path,
            "[Info] [PrismLauncher] Creating instance directory...",
        )?;

        println!("[Info] Found PrismLauncher directory");

        let dir_path = format!("{}{}", prefix, ICONS_DIR_PATH);
        mk_dir(
            &dir_path,
            "[Info] [PrismLauncher] Creating icons directory...",
        )?;

        let icon_path = format!("{}{}", prefix, ICON_PATH);
        if !Path::new(&icon_path).exists() {
            println!("[Info] [PrismLauncher] Copying icon...");
            fs::write(&icon_path, include_bytes!("../resources/leafish-icon.png"))?;
        }

        let cfg_path = format!("{}{}", prefix, CFG_PATH);
        fs::write(&cfg_path, DEFAULT_CFG.as_bytes())?;

        let pack_path = format!("{}{}", prefix, PACK_PATH);
        fs::write(
            &pack_path,
            serde_json::to_string(&PackDesc {
                components: vec![Component {
                    important: Some(true),
                    uid: "de.leafish".to_string(),
                    version: "Leafish".to_string(),
                }],
                format_version: 1,
            })?,
        )?;

        Ok(true)
    }

    #[derive(Serialize)]
    struct PackDesc {
        components: Vec<Component>,
        #[serde(rename = "formatVersion")]
        format_version: usize,
    }

    #[skip_serializing_none]
    #[derive(Serialize)]
    struct Component {
        #[serde(rename = "cachedName")]
        cached_name: Option<String>,
        #[serde(rename = "cachedRequires")]
        cached_requires: Option<Required>,
        #[serde(rename = "cachedVersion")]
        cached_version: Option<String>,
        #[serde(rename = "cachedVolatile")]
        cached_volatile: Option<bool>,
        #[serde(rename = "dependencyOnly")]
        dependency_only: Option<bool>,
        important: Option<bool>,
        uid: String,
        version: String,
    }

    #[skip_serializing_none]
    #[derive(Serialize)]
    struct VersionMeta {
        #[serde(rename = "+traits")]
        traits: Option<Vec<String>>,
        #[serde(rename = "assetIndex")]
        asset_index: AssetIndex,
        #[serde(rename = "compatibleJavaMajors")]
        compatible_java_majors: Vec<usize>,
        #[serde(rename = "formatVersion")]
        format_version: usize,
        libraries: Vec<Library>,
        #[serde(rename = "mainClass")]
        main_class: String,
        #[serde(rename = "mainJar")]
        main_jar: MainJar,
        #[serde(rename = "minecraftArguments")]
        minecraft_arguments: String,
        name: String,
        order: isize,
        #[serde(rename = "releaseTime")]
        release_time: String,
        requires: Vec<String>,
        #[serde(rename = "type")]
        ty: String,
        uid: String,
        version: String,
    }

    #[derive(Serialize)]
    struct MainJar {
        downloads: Download,
        name: String,
    }

    #[derive(Serialize)]
    struct AssetIndex {
        id: String,
        sha1: String,
        size: usize,
        #[serde(rename = "totalSize")]
        total_size: usize,
        url: String,
    }

    #[derive(Serialize)]
    struct Library {
        downloads: Vec<Download>,
        name: String,
    }

    #[derive(Serialize)]
    struct Download {
        artifact: Artifact,
    }

    #[derive(Serialize)]
    struct Artifact {
        sha1: String,
        size: usize,
        url: String,
    }

    #[derive(Serialize)]
    struct Required {
        suggests: String,
        uid: String,
    }

}

fn mk_dir(path: &str, msg: &str) -> anyhow::Result<()> {
    if !Path::new(path).exists() {
        println!("{}", msg);
        fs::create_dir_all(path)?;
    }
    Ok(())
}
