// Copyright 2016 Matthew Collins
// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::paths;

use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use crate::format::{Color, Component, ComponentType};
use crate::render;
use crate::ui;
use parking_lot::Mutex;

const FILTERED_CRATES: &[&str] = &[
    //"reqwest", // TODO: needed?
    "mime",
];

pub struct CVar<T: Sized + Any + 'static> {
    pub name: &'static str,
    pub ty: PhantomData<T>,
    pub description: &'static str,
    pub mutable: bool,
    pub serializable: bool,
    pub default: &'static dyn Fn() -> T,
}

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

impl Var for CVar<i64> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        val.downcast_ref::<i64>().unwrap().to_string()
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new(input.parse::<i64>().unwrap())
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

impl Var for CVar<bool> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        val.downcast_ref::<bool>().unwrap().to_string()
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new(input.parse::<bool>().unwrap())
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

impl Var for CVar<String> {
    fn serialize(&self, val: &Box<dyn Any>) -> String {
        format!("\"{}\"", val.downcast_ref::<String>().unwrap())
    }

    fn deserialize(&self, input: &str) -> Box<dyn Any> {
        Box::new((&input[1..input.len() - 1]).to_owned())
    }

    fn description(&self) -> &'static str {
        self.description
    }
    fn can_serialize(&self) -> bool {
        self.serializable
    }
}

pub trait Var {
    fn serialize(&self, val: &Box<dyn Any>) -> String;
    fn deserialize(&self, input: &str) -> Box<dyn Any>;
    fn description(&self) -> &'static str;
    fn can_serialize(&self) -> bool;
}

#[derive(Default)]
pub struct Vars {
    names: HashMap<String, &'static str>,
    vars: HashMap<&'static str, Box<dyn Var>>,
    var_values: HashMap<&'static str, RefCell<Box<dyn Any>>>,
}

impl Vars {
    pub fn new() -> Vars {
        Default::default()
    }

    pub fn register<T: Sized + Any>(&mut self, var: CVar<T>)
    where
        CVar<T>: Var,
    {
        if self.vars.contains_key(var.name) {
            panic!("Key registered twice {}", var.name);
        }
        self.names.insert(var.name.to_owned(), var.name);
        self.var_values
            .insert(var.name, RefCell::new(Box::new((var.default)())));
        self.vars.insert(var.name, Box::new(var));
    }

    pub fn get<T: Sized + Any>(&self, var: CVar<T>) -> Ref<T>
    where
        CVar<T>: Var,
    {
        // Should never fail
        let var = self.var_values.get(var.name).unwrap().borrow();
        Ref::map(var, |v| v.downcast_ref::<T>().unwrap())
    }

    pub fn set<T: Sized + Any>(&self, var: CVar<T>, val: T)
    where
        CVar<T>: Var,
    {
        *self.var_values.get(var.name).unwrap().borrow_mut() = Box::new(val);
        self.save_config();
    }

    pub fn load_config(&mut self) {
        if let Ok(file) = fs::File::open(paths::get_config_dir().join("conf.cfg")) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line.unwrap();
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                let parts = line
                    .splitn(2, ' ')
                    .map(|v| v.to_owned())
                    .collect::<Vec<String>>();
                let (name, arg) = (&parts[0], &parts[1]);
                if let Some(var_name) = self.names.get(name) {
                    let var = self.vars.get(var_name).unwrap();
                    let val = var.deserialize(arg);
                    if var.can_serialize() {
                        self.var_values.insert(var_name, RefCell::new(val));
                    }
                }
            }
        }
    }

    pub fn save_config(&self) {
        let mut file =
            BufWriter::new(fs::File::create(paths::get_config_dir().join("conf.cfg")).unwrap());
        for (name, var) in &self.vars {
            if !var.can_serialize() {
                continue;
            }
            for line in var.description().lines() {
                writeln!(file, "# {}", line).unwrap();
            }
            write!(
                file,
                "{} {}\n\n",
                name,
                var.serialize(&self.var_values.get(name).unwrap().borrow())
            )
            .unwrap();
        }
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

    pub fn configure(&mut self, vars: &Vars) {
        self.log_level_term = log_level_from_str(&vars.get(LOG_LEVEL_TERM), log::Level::Info);
        self.log_level_file = log_level_from_str(&vars.get(LOG_LEVEL_FILE), log::Level::Debug);

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
