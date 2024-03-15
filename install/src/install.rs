use std::{
    fs::{self, File},
    path::Path,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const BOOTSTRAP_BIN: &[u8] = include_bytes!("../resources/bootstrap_x86_64_linux");
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const BOOTSTRAP_BIN: &[u8] = include_bytes!("../resources/bootstrap_x86_64_windows.exe");

const BOOTSTRAP_JAR: &[u8] = include_bytes!("../resources/wrapper.jar");

pub mod mojang {

    use std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::{Read, Seek, Write},
        path::Path,
    };

    use chrono::Utc;
    use serde::{Deserialize, Serialize};
    use serde_with::skip_serializing_none;

    use crate::install::{adjust_perms, mk_dir, BOOTSTRAP_BIN, BOOTSTRAP_JAR};

    // all of these are expected to be prepended with ".minecraft"
    const DIR_PATH: &str = "/versions/Leafish/";
    const DESC_JSON_PATH: &str = "/versions/Leafish/Leafish.json";
    const JAR_PATH: &str = "/versions/Leafish/Leafish.jar";
    const PROFILES_JSON_PATH: &str = "/launcher_profiles.json";
    const LIBRARY_DIR_PATH: &str = "/libraries/leafish/Leafish/Jar/";
    const LIBRARY_PATH: &str = "/libraries/leafish/Leafish/Jar/Leafish-Jar.jar";

    #[cfg(target_os = "windows")]
    const BOOTSTRAP_BIN_PATH: &str = "/versions/Leafish/bootstrap.exe";
    #[cfg(not(target_os = "windows"))]
    const BOOTSTRAP_BIN_PATH: &str = "/versions/Leafish/bootstrap";

    // FIXME: add cli that allows reinstalling, uninstalling, installing and getting info

    #[derive(Serialize)]
    struct Description {
        id: String,
        #[serde(rename = "assetIndex")]
        asset_index: AssetIndex,
        assets: String,
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

    #[derive(Serialize)]
    struct AssetIndex {
        id: String,
        sha1: String,
        size: usize,
        #[serde(rename = "totalSize")]
        total_size: usize,
        url: String,
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
            println!("[Info] [Official] Couldn't find .minecraft directory");
            return Ok(false);
        }

        let json_path = format!("{}{}", prefix, DESC_JSON_PATH);
        let jar_path = format!("{}{}", prefix, JAR_PATH);
        if Path::new(&json_path).exists() && Path::new(&jar_path).exists() {
            println!("[Info] [Official] Leafish is already installed");
            return Ok(false);
        }
        // cleanup old files
        if Path::new(&json_path).exists() {
            println!("[Info] [Official] Removing old json...");
            fs::remove_file(&json_path)?;
        }
        if Path::new(&jar_path).exists() {
            println!("[Info] [Official] Removing old jar...");
            fs::remove_file(&jar_path)?;
        }

        let dir_path = format!("{}{}", prefix, DIR_PATH);
        // create version directory if necessary
        mk_dir(&dir_path, "[Info] [Official] Creating version directory...")?;

        // write files
        println!("[Info] [Official] Creating json...");
        let mut json = File::create_new(&json_path)?;
        json.write_all(serde_json::to_string_pretty(&Description {
            id: "Leafish".to_string(),
            time: "2020-01-01T00:00:00+02:00".to_string(), // FIXME: use some sensible time
            release_time: "2020-01-01T00:00:00+02:00".to_string(),
            ty: "release".to_string(),
            libraries: vec![Library { name: "leafish:Leafish:Jar".to_string() }], // we need nobody, but ourselves ;)
            main_class: "de.leafish.Main".to_string(),
            minecraft_arguments: "--username ${auth_player_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type} --launcher official".to_string(),
            asset_index: AssetIndex { // FIXME: don't choose one version statically!
                id: "1.19".to_string(),
                sha1: "a9c8b05a8082a65678beda6dfa2b8f21fa627bce".to_string(),
                size: 385608,
                total_size: 557023211, // FIXME: does this not depend on the client jar?
                url: "https://piston-meta.mojang.com/v1/packages/a9c8b05a8082a65678beda6dfa2b8f21fa627bce/1.19.json".to_string(),
            },
            assets: "1.19".to_string(),
        })?.as_bytes())?;

        // FIXME: download the latest jar from github

        println!("[Info] [Official] Copying bootstrap jar...");
        let mut jar = File::create_new(&jar_path)?;
        jar.write_all(BOOTSTRAP_JAR)?;

        println!("[Info] [Official] Copying bootstrap binary...");
        let mut file = File::create_new(format!("{}{}", prefix, BOOTSTRAP_BIN_PATH))?;
        file.write_all(BOOTSTRAP_BIN)?;
        adjust_perms(&file)?;

        let lib_dir = format!("{}{}", prefix, LIBRARY_DIR_PATH);
        if !Path::new(&lib_dir).exists() {
            println!("[Info] [Official] Copying library...");
            fs::create_dir_all(&lib_dir)?;
            fs::write(format!("{}{}", prefix, LIBRARY_PATH), BOOTSTRAP_JAR)?;
        }

        let profiles_path = format!("{}{}", prefix, PROFILES_JSON_PATH);
        if !Path::new(&profiles_path).exists() {
            println!(
                "[Warn] [Official] Couldn't create profile as the file {} doesn't exist",
                profiles_path
            );
            return Ok(true);
        }
        println!("[Info] [Official] Installing profile...");
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

        println!("[Info] [Official] Installation into .minecraft directory successful");

        Ok(true)
    }
}

pub mod prism {
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    use chrono::Utc;
    use serde::Serialize;
    use serde_with::skip_serializing_none;

    use crate::install::{adjust_perms, mk_dir, BOOTSTRAP_BIN, BOOTSTRAP_JAR};

    const ICONS_DIR_PATH: &str = "/icons";
    const ICON_PATH: &str = "/icons/leafish.png";
    const INSTANCE_DIR_PATH: &str = "/instances/Leafish";
    const CFG_PATH: &str = "/instances/Leafish/instance.cfg";
    const PACK_PATH: &str = "/instances/Leafish/mmc-pack.json";
    const META_DIR_PATH: &str = "/meta/de.leafish";
    const META_PATH: &str = "/meta/de.leafish/Leafish.json";
    const LIB_DIR_PATH: &str = "/libraries/de/leafish";
    const LIB_PATH: &str = "/libraries/de/leafish/Leafish.jar";

    #[cfg(not(target_os = "windows"))]
    const BOOTSTRAP_BIN_PATH: &str = "/instances/Leafish/bootstrap";
    #[cfg(target_os = "windows")]
    const BOOTSTRAP_BIN_PATH: &str = "/instances/Leafish/bootstrap.exe";

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

        println!("[Info] [PrismLauncher] Found PrismLauncher directory");

        let dir_path = format!("{}{}", prefix, INSTANCE_DIR_PATH);
        if Path::new(&dir_path).exists() {
            println!("[Info] [PrismLauncher] Profile already exists");
            return Ok(false);
        }
        mk_dir(
            &dir_path,
            "[Info] [PrismLauncher] Creating instance directory...",
        )?;

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
        fs::write(cfg_path, DEFAULT_CFG.as_bytes())?;

        let pack_path = format!("{}{}", prefix, PACK_PATH);
        fs::write(
            pack_path,
            serde_json::to_string_pretty(&PackDesc {
                components: vec![Component {
                    important: Some(true),
                    uid: "de.leafish".to_string(),
                    version: "Leafish".to_string(),
                    // cached_name: Some("Leafish".to_string()),
                    cached_name: None,
                    cached_requires: None,
                    // cached_version: Some("Leafish".to_string()),
                    cached_version: None,
                    cached_volatile: None,
                    dependency_only: None,
                }],
                format_version: 1,
            })?,
        )?;
        let dir_path = format!("{}{}", prefix, LIB_DIR_PATH);
        mk_dir(
            &dir_path,
            "[Info] [PrismLauncher] Creating library directory...",
        )?;
        let lib_path = format!("{}{}", prefix, LIB_PATH);
        fs::write(lib_path, BOOTSTRAP_JAR)?;

        println!("[Info] [PrismLauncher] Copying bootstrap binary...");
        let mut bootstrap_binary = File::create_new(format!("{}{}", prefix, BOOTSTRAP_BIN_PATH))?;
        bootstrap_binary.write_all(BOOTSTRAP_BIN)?;
        adjust_perms(&bootstrap_binary)?;
        let dir_path = format!("{}{}", prefix, META_DIR_PATH);
        mk_dir(
            &dir_path,
            "[Info] [PrismLauncher] Creating meta directory...",
        )?;
        let meta_path = format!("{}{}", prefix, META_PATH);

        let now = Utc::now();
        let now = now.format("%Y-%m-%dT%H:%M:%S");

        fs::write(meta_path, serde_json::to_string_pretty(&VersionMeta {
            traits: None,
            asset_index: AssetIndex { // FIXME: don't choose one version statically!
                id: "1.19".to_string(),
                sha1: "a9c8b05a8082a65678beda6dfa2b8f21fa627bce".to_string(),
                size: 385608,
                total_size: 557023211, // FIXME: does this not depend on the client jar?
                url: "https://piston-meta.mojang.com/v1/packages/a9c8b05a8082a65678beda6dfa2b8f21fa627bce/1.19.json".to_string(),
            },
            compatible_java_majors: vec![8,9,10,11,12,13,14,15,16,17],
            format_version: 1,
            libraries: vec![],
            main_class: "de.leafish.Main".to_string(),
            main_jar: MainJar {
                downloads: Download {
                    artifact: Some(Artifact {
                        sha1: "fdfbab865254c28e67a6c4e7448583147db3a7f2".to_string(),
                        size: 5118199,
                        url: "https://github.com/Lea-fish/Releases/releases/download/alpha/bootstrap.jar".to_string(),
                    }),
                },
                name: "de.leafish:Leafish:v1.0.0".to_string(),
            },
            minecraft_arguments: "--username ${auth_player_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type} --path ../".to_string(),
            name: "Leafish".to_string(),
            order: 0,
            release_time: now.to_string(),
            requires: vec![],
            ty: "release".to_string(),
            uid: "net.minecraft".to_string(),
            version: "1.19.2".to_string(),
        })?)?;

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

    #[skip_serializing_none]
    #[derive(Serialize)]
    struct Download {
        artifact: Option<Artifact>,
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

#[cfg(target_os = "linux")]
fn adjust_perms(file: &File) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o777);
    file.set_permissions(perms)?;
    Ok(())
}

#[inline]
#[cfg(not(target_os = "linux"))]
fn adjust_perms(_file: &File) -> anyhow::Result<()> {
    Ok(())
}
