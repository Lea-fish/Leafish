use install::setup_launcher_wrapper;

mod install;

fn main() {
    let dir = mc_dir();
    setup_launcher_wrapper(&dir).unwrap();
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn mc_dir() -> String {
    platform_dirs::AppDirs::new(Some(".minecraft"), false).unwrap().config_dir.to_str().unwrap().to_string()
}

#[cfg(target_os = "linux")]
fn mc_dir() -> String {
    let state = platform_dirs::AppDirs::new(None, false).unwrap();
    let full = state.config_dir.to_str().unwrap();
    format!("{}{}", &full[0..(full.len() - ".config".len())], ".minecraft")
}
