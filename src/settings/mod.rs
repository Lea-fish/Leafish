use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
mod config_var;
use log::{info, warn};
use parking_lot::Mutex;
mod keybinds;
use crate::paths;
pub use config_var::*;
pub use keybinds::*;

pub const DOUBLE_JUMP_MS: u32 = 100;

// stores all game settings, except keybinds
pub struct SettingStore(Mutex<HashMap<SettingType, ConfigVar>>);

impl SettingStore {
    pub fn new() -> Self {
        let mut store = SettingStore(Mutex::new(HashMap::new()));
        store.load_defaults();
        store.load_config();
        store
    }

    pub fn set(&self, s_type: SettingType, val: SettingValue) {
        self.0.lock().get_mut(&s_type).unwrap().value = val;
        self.save_config();
    }

    pub fn get_value(&self, input: SettingType) -> SettingValue {
        self.0.lock().get(&input).unwrap().value.clone()
    }

    pub fn get_bool(&self, input: SettingType) -> bool {
        self.0
            .lock()
            .get(&input)
            .map(|v| v.as_bool())
            .flatten()
            .unwrap()
    }

    pub fn get_i32(&self, input: SettingType) -> i32 {
        self.0
            .lock()
            .get(&input)
            .map(|v| v.as_i32())
            .flatten()
            .unwrap()
    }

    pub fn get_float(&self, input: SettingType) -> f64 {
        self.0
            .lock()
            .get(&input)
            .map(|v| v.as_float())
            .flatten()
            .unwrap()
    }

    pub fn get_string(&self, input: SettingType) -> String {
        self.0
            .lock()
            .get(&input)
            .map(|v| v.as_string())
            .flatten()
            .unwrap()
    }

    fn load_config(&mut self) {
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
                if name.starts_with("keybind_") {
                    continue;
                }
                let mut store = self.0.lock();
                if let Some((s_type, setting)) = store.clone().iter().find(|(_, e)| e.name == name)
                {
                    let val = deserialize_value(arg);
                    // just a check if the SettingType changed in config file
                    if std::mem::discriminant(&val) != std::mem::discriminant(&setting.value) {
                        warn!("a setting had a different type in config file than default type: {name}");
                    }
                    if setting.serializable {
                        store.get_mut(s_type).unwrap().value = val;
                    }
                } else {
                    info!("a unknwon config option was specified: {name}");
                }
            }
        }
    }

    fn save_config(&self) {
        let mut file =
            BufWriter::new(fs::File::create(paths::get_config_dir().join("conf.cfg")).unwrap());
        for var in self.0.lock().values() {
            if !var.serializable {
                continue;
            }
            for line in var.description.lines() {
                if let Err(err) = writeln!(file, "# {}", line) {
                    warn!("couldnt write a setting description to config file: {err}, {line}");
                }
            }
            let name = var.name;

            if let Err(err) = match &var.value {
                SettingValue::Float(f) => write!(file, "{name} {f}\n\n"),
                SettingValue::Num(n) => write!(file, "{name} {n}\n\n"),
                SettingValue::Bool(b) => write!(file, "{name} {b}\n\n"),
                SettingValue::String(s) => write!(file, "{name} {s}\n\n"),
            } {
                warn!("couldnt write a setting to config file: {err}, {name}");
            }
        }
    }

    fn load_defaults(&self) {
        let mut s = self.0.lock();
        for (var_type, var) in default_vars() {
            s.insert(var_type, var);
        }
    }
}

fn deserialize_value(input: &str) -> SettingValue {
    if let Ok(num) = input.parse::<i32>() {
        SettingValue::Num(num)
    } else if let Ok(float) = input.parse::<f64>() {
        SettingValue::Float(float)
    } else if let Ok(bool) = input.parse::<bool>() {
        SettingValue::Bool(bool)
    } else {
        SettingValue::String(input.to_owned())
    }
}
