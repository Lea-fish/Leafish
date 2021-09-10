// Copyright 2016 Matthew Collins
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::paths;
use crate::settings;
use crate::settings::{CVar, Vars};

use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::format::{Color, Component, TextComponent};
use crate::render;
use crate::ui;
use parking_lot::Mutex;
use parking_lot::RwLock;

const FILTERED_CRATES: &[&str] = &[
    //"reqwest", // TODO: needed?
    "mime",
];

pub const LOG_LEVEL_TERM: CVar<String> = CVar {
    ty: PhantomData,
    name: "log_level_term",
    description: "log level of messages to log to the terminal",
    mutable: false,
    serializable: true,
    default: &|| "info".to_owned(),
};

pub const LOG_LEVEL_FILE: CVar<String> = CVar {
    ty: PhantomData,
    name: "log_level_file",
    description: "log level of messages to log to the log file",
    mutable: false,
    serializable: true,
    default: &|| "trace".to_owned(),
};

pub fn register_vars(vars: &mut Vars) {
    vars.register(LOG_LEVEL_TERM);
    vars.register(LOG_LEVEL_FILE);
}

fn log_level_from_str(s: &str, default: log::Level) -> log::Level {
    // TODO: no opposite of FromStr in log crate?
    use log::Level::*;
    match s {
        "trace" => Trace,
        "debug" => Debug,
        "info" => Info,
        "warn" => Warn,
        "error" => Error,
        _ => default,
    }
}

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
            history: vec![Component::Text(TextComponent::new("")); 200],
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

    pub fn configure(&mut self) {
        let vars = settings::Vars::new();
        self.log_level_term = log_level_from_str(&vars.get(LOG_LEVEL_TERM), log::Level::Info);
        self.log_level_file = log_level_from_str(&vars.get(LOG_LEVEL_FILE), log::Level::Trace);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn tick(
        &mut self,
        ui_container: &mut ui::Container,
        renderer: Arc<RwLock<render::Renderer>>,
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
            let renderer = &*renderer.read();
            for line in self.history.iter().rev() {
                if offset >= 210.0 {
                    break;
                }
                let (_, height) = ui::Formatted::compute_size(renderer, line, w - 10.0);
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

        let mut file = &record.file().unwrap_or("").replace("\\", "/")[..];
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
        }

        self.history.remove(0);
        let mut msg = TextComponent::new("");
        msg.modifier.extra = Some(vec![
            Component::Text(TextComponent::new("[")),
            {
                let mut msg = TextComponent::new(file);
                msg.modifier.color = Some(Color::Green);
                Component::Text(msg)
            },
            Component::Text(TextComponent::new(":")),
            {
                let mut msg = TextComponent::new(&format!("{}", record.line().unwrap_or(0)));
                msg.modifier.color = Some(Color::Aqua);
                Component::Text(msg)
            },
            Component::Text(TextComponent::new("]")),
            Component::Text(TextComponent::new("[")),
            {
                let mut msg = TextComponent::new(&format!("{}", record.level()));
                msg.modifier.color = Some(match record.level() {
                    log::Level::Debug => Color::Green,
                    log::Level::Error => Color::Red,
                    log::Level::Warn => Color::Yellow,
                    log::Level::Info => Color::Aqua,
                    log::Level::Trace => Color::Blue,
                });
                Component::Text(msg)
            },
            Component::Text(TextComponent::new("] ")),
            Component::Text(TextComponent::new(&format!("{}", record.args()))),
        ]);
        self.history.push(Component::Text(msg));
        self.dirty = true;
    }
}

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
