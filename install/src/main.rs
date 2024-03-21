use install::{mojang, prism};

mod install;

fn main() {
    let mojang_dir = mojang_dir();
    mojang::setup(&mojang_dir).unwrap();
    for dir in prism_dirs() {
        prism::setup(&dir).unwrap();
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn mojang_dir() -> String {
    platform_dirs::AppDirs::new(Some(".minecraft"), false)
        .unwrap()
        .config_dir
        .to_str()
        .unwrap()
        .to_string()
}

#[cfg(target_os = "linux")]
fn mojang_dir() -> String {
    let state = platform_dirs::AppDirs::new(None, false).unwrap();
    let full = state.config_dir.to_str().unwrap();
    format!(
        "{}{}",
        &full[0..(full.len() - ".config".len())],
        ".minecraft"
    )
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn prism_dirs() -> Vec<String> {
    vec![platform_dirs::AppDirs::new(Some("PrismLauncher"), false)
        .unwrap()
        .config_dir
        .to_str()
        .unwrap()
        .to_string()]
}

#[cfg(target_os = "linux")]
fn prism_dirs() -> Vec<String> {
    let flatpak = {
        let state = platform_dirs::AppDirs::new(None, false).unwrap();
        let full = state.config_dir.to_str().unwrap();
        format!(
            "{}{}",
            &full[0..(full.len() - ".config".len())],
            ".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"
        )
    };
    let share = {
        let state = platform_dirs::AppDirs::new(None, false).unwrap();
        let full = state.config_dir.to_str().unwrap();
        format!(
            "{}{}",
            &full[0..(full.len() - ".config".len())],
            ".local/share/PrismLauncher"
        )
    };
    vec![
        platform_dirs::AppDirs::new(None, false)
            .unwrap()
            .data_dir
            .to_str()
            .unwrap()
            .to_string(),
        flatpak,
        share,
    ]
}
