use log::Level;
use parking_lot::Mutex;

use crate::format::{Color, Component, ComponentType};
use crate::render;
use crate::settings::{SettingStore, SettingType};
use crate::{paths, ui};

use std::fs;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;

pub struct Console {
    history: Vec<Component>,
    dirty: bool,
    logfile: fs::File,
    log_level_term: log::Level,
    log_level_file: log::Level,

    elements: Option<ConsoleElements>,
    active: bool,
    position: f64,
}

struct ConsoleElements {
    background: ui::ImageRef,
    lines: Vec<ui::FormattedRef>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Console {
        Console {
            history: vec![Component::new(ComponentType::new("", None)); 200],
            dirty: false,
            logfile: fs::File::create(paths::get_cache_dir().join("client.log"))
                .expect("failed to open log file"),
            log_level_term: log::Level::Info,
            log_level_file: log::Level::Trace,

            elements: None,
            active: false,
            position: -220.0,
        }
    }

    fn log_level_from_env(name: &str) -> Option<log::Level> {
        let variable_string = std::env::var(name).ok()?;
        log::Level::from_str(&variable_string).ok()
    }

    pub fn configure(&mut self, settings: &SettingStore) {
        self.log_level_term = term_log_level(settings).unwrap_or(Level::Info);
        self.log_level_file = file_log_level(settings).unwrap_or(Level::Debug);

        for name in ["RUST_LOG", "LOG_LEVEL"].iter() {
            if let Some(level) = Console::log_level_from_env(name) {
                self.log_level_term = level;
                self.log_level_file = level;
            }
        }
        if let Some(level) = Console::log_level_from_env("RUST_LOG") {
            self.log_level_term = level;
        }
        if let Some(level) = Console::log_level_from_env("LOG_LEVEL_FILE") {
            self.log_level_file = level;
        }
    }

    pub fn _is_active(&self) -> bool {
        self.active
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn _activate(&mut self) {
        self.active = true;
    }

    pub fn tick(
        &mut self,
        ui_container: &mut ui::Container,
        renderer: Arc<render::Renderer>,
        delta: f64,
        width: f64,
    ) {
        if !self.active && self.position <= -220.0 {
            self.elements = None;
            return;
        }
        if self.active {
            if self.position < 0.0 {
                self.position += delta * 4.0;
            } else {
                self.position = 0.0;
            }
        } else if self.position > -220.0 {
            self.position -= delta * 4.0;
        } else {
            self.position = -220.0;
        }

        let w = match ui_container.mode {
            ui::Mode::Scaled => width,
            ui::Mode::Unscaled(scale) => 854.0 / scale,
        };
        if self.elements.is_none() {
            let background = ui::ImageBuilder::new()
                .texture("leafish:solid")
                .position(0.0, self.position)
                .size(w, 220.0)
                .colour((0, 0, 0, 180))
                .draw_index(500)
                .create(ui_container);
            self.elements = Some(ConsoleElements {
                background,
                lines: vec![],
            });
            self.dirty = true;
        }
        let elements = self.elements.as_mut().unwrap();
        let mut background = elements.background.borrow_mut();
        background.y = self.position;
        background.width = w;

        if self.dirty {
            self.dirty = false;
            elements.lines.clear();

            let mut offset = 0.0;
            for line in self.history.iter().rev() {
                if offset >= 210.0 {
                    break;
                }
                let (_, height) =
                    ui::Formatted::compute_size(renderer.clone(), line, w - 10.0, 1.0, 1.0, 1.0);
                elements.lines.push(
                    ui::FormattedBuilder::new()
                        .text(line.clone())
                        .position(5.0, 5.0 + offset)
                        .max_width(w - 10.0)
                        .alignment(ui::VAttach::Bottom, ui::HAttach::Left)
                        .create(&mut *background),
                );
                offset += height;
            }
        }
    }

    fn log(&mut self, record: &log::Record) {
        for filtered in FILTERED_CRATES {
            if record.module_path().unwrap_or("").starts_with(filtered) {
                return;
            }
        }

        let mut file = &record.file().unwrap_or("").replace('\\', "/")[..];
        if let Some(pos) = file.rfind("src/") {
            file = &file[pos + 4..];
        }

        let line = format!(
            "[{}:{}][{}] {}",
            file,
            record.line().unwrap_or(0),
            record.level(),
            record.args()
        );

        if record.level() <= self.log_level_file {
            self.logfile.write_all(line.as_bytes()).unwrap();
            self.logfile.write_all(b"\n").unwrap();
        }

        if record.level() <= self.log_level_term {
            println!("{}", line);

            self.history.remove(0);
            let component = Component {
                list: vec![
                    ComponentType::new("[", None),
                    ComponentType::new(file, Some(Color::Green)),
                    ComponentType::new(":", None),
                    ComponentType::new(
                        &format!("{}", record.line().unwrap_or(0)),
                        Some(Color::Aqua),
                    ),
                    ComponentType::new("]", None),
                    ComponentType::new("[", None),
                    ComponentType::new(
                        &format!("{}", record.level()),
                        Some(match record.level() {
                            log::Level::Debug => Color::Green,
                            log::Level::Error => Color::Red,
                            log::Level::Warn => Color::Yellow,
                            log::Level::Info => Color::Aqua,
                            log::Level::Trace => Color::Blue,
                        }),
                    ),
                    ComponentType::new("] ", None),
                    ComponentType::new(&format!("{}", record.args()), None),
                ],
            };
            self.history.push(component);
            self.dirty = true;
        }
    }
}

fn _log_level_from_str(s: &str) -> Option<log::Level> {
    // TODO: no opposite of FromStr in log crate?
    use log::Level::*;
    match s {
        "trace" => Some(Trace),
        "debug" => Some(Debug),
        "info" => Some(Info),
        "warn" => Some(Warn),
        "error" => Some(Error),
        _ => None,
    }
}

fn term_log_level(store: &SettingStore) -> Option<Level> {
    let val = store.get_string(SettingType::LogLevelTerm);
    Level::from_str(&val).ok()
}
fn file_log_level(store: &SettingStore) -> Option<Level> {
    let val = store.get_string(SettingType::LogLevelFile);
    Level::from_str(&val).ok()
}

const FILTERED_CRATES: &[&str] = &[
    //"reqwest", // TODO: needed?
    "mime",
];

pub struct ConsoleProxy {
    console: Arc<Mutex<Console>>,
}

impl ConsoleProxy {
    pub fn new(con: Arc<Mutex<Console>>) -> ConsoleProxy {
        ConsoleProxy { console: con }
    }
}

impl log::Log for ConsoleProxy {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.console.lock().log(record);
        }
    }

    fn flush(&self) {}
}

unsafe impl Send for ConsoleProxy {}
unsafe impl Sync for ConsoleProxy {}
