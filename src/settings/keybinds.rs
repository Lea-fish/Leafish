use arc_swap::ArcSwap;
use log::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use winit::platform::scancode::PhysicalKeyExtScancode;

use winit::keyboard::{Key, KeyCode};

use crate::paths;

use super::default_keybinds::create_keybinds;

#[derive(Clone, Copy)]
pub struct Keybind {
    pub name: &'static str,
    pub description: &'static str,
    pub action: Actionkey,
}

pub struct KeybindStore {
    key_cache: ArcSwap<HashMap<Key, Keybind>>,
    mapping_cache: ArcSwap<HashMap<u32, Keybind>>,
}

impl KeybindStore {
    pub fn new() -> Self {
        let mut store = KeybindStore {
            key_cache: ArcSwap::new(Arc::new(HashMap::new())),
            mapping_cache: ArcSwap::new(Arc::new(HashMap::new())),
        };
        store.load_defaults();
        store.load_config();
        store.save_config();
        store
    }

    pub fn get(&self, code: KeyCode, key: &Key) -> Option<Keybind> {
        if let Some(cache) = self
            .mapping_cache
            .load()
            .get(&code.to_scancode().unwrap())
            .copied()
        {
            return Some(cache);
        }
        if let Some(cached) = self.key_cache.load().get(key) {
            let mut cache = self.mapping_cache.load().deref().deref().clone();
            cache.insert(code.to_scancode().unwrap(), *cached);
            return Some(*cached);
        }
        None
    }

    pub fn set(&self, key: Key, action: Actionkey) {
        let old_key = self
            .key_cache
            .load()
            .iter()
            .find(|(_, v)| v.action == action)
            .expect("a action was not bound to a key?")
            .0
            .clone();
        let old_mapping = self.mapping_cache.load();
        let old_raw_keys = old_mapping
            .iter()
            .filter(|(_, v)| v.action == action)
            .collect::<Vec<_>>();
        let mut mapping = self.mapping_cache.load().deref().deref().clone();
        for key in old_raw_keys {
            mapping.remove(key.0);
        }

        let mut cache = self.key_cache.load().deref().deref().clone();
        let old_val = cache.remove(&old_key).unwrap();
        cache.insert(key, old_val);
        self.key_cache.store(Arc::new(cache));
        self.save_config();
    }

    fn load_config(&mut self) {
        if let Ok(file) = fs::File::open(paths::get_config_dir().join("keybinds.cfg")) {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let Ok(line) = line else {
                    warn!("failed reading a line in the config file");
                    continue;
                };
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                let parts = line
                    .splitn(2, ' ')
                    .map(|v| v.to_owned())
                    .collect::<Vec<String>>();
                let (name, arg) = (&parts[0], &parts[1]);
                if !name.starts_with("keybind_") {
                    continue;
                }
                let mut store = self.key_cache.load().deref().deref().clone();
                if let Some(action) = store
                    .values()
                    .find(|v| Actionkey::from_str(name).is_ok_and(|k| k == v.action))
                {
                    if let Some(new_key) = deserialize_key(arg) {
                        let key = store
                            .iter()
                            .find(|(_, v)| v.action == action.action)
                            .expect("a action was not bound to a key?")
                            .0
                            .clone();

                        let old_val = store.remove(&key).unwrap();
                        store.insert(new_key, old_val);
                    }
                } else {
                    info!("an unknown keybind was specified: {name}");
                }
            }
        }
    }

    fn save_config(&self) {
        let mut file =
            BufWriter::new(fs::File::create(paths::get_config_dir().join("keybinds.cfg")).unwrap());
        for (key, keybind) in self.key_cache.load().iter() {
            for line in keybind.description.lines() {
                if let Err(err) = writeln!(file, "# {}", line) {
                    warn!(
                        "couldnt write a keybind description to config file {err}, {}",
                        keybind.name
                    );
                }
            }
            if let Err(err) = write!(file, "{} {:?}\n\n", keybind.name, key.clone()) {
                warn!(
                    "couldnt write a keybind to config file {err}, {}",
                    keybind.name
                );
            };
        }
    }

    fn load_defaults(&self) {
        let mut s = self.key_cache.load().deref().deref().clone();
        for bind in create_keybinds() {
            s.insert(bind.0, bind.1);
        }
        self.key_cache.store(Arc::new(s));
    }
}

fn deserialize_key(input: &str) -> Option<Key> {
    match serde_json::from_str(input) {
        Ok(num) => Some(num),
        Err(err) => {
            warn!("couldnt deserialize keybind: {err}, {input}");
            None
        }
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum Actionkey {
    Forward,
    Backward,
    Left,
    Right,
    OpenInv,
    Sneak,
    Sprint,
    Jump,
    ToggleHud,
    ToggleDebug,
    ToggleChat,
}

impl FromStr for Actionkey {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keybind_forward" => Ok(Actionkey::Forward),
            "keybind_backward" => Ok(Actionkey::Backward),
            "keybind_left" => Ok(Actionkey::Left),
            "keybind_right" => Ok(Actionkey::Right),
            "keybind_open_inv" => Ok(Actionkey::OpenInv),
            "keybind_sneak" => Ok(Actionkey::Sneak),
            "keybind_sprint" => Ok(Actionkey::Sprint),
            "keybind_jump" => Ok(Actionkey::Jump),
            "keybind_toggle_hud" => Ok(Actionkey::ToggleHud),
            "keybind_toggle_debug_info" => Ok(Actionkey::ToggleDebug),
            "keybind_toggle_chat" => Ok(Actionkey::ToggleChat),
            _ => Err(()),
        }
    }
}

impl Actionkey {
    const VALUES: [Actionkey; 11] = [
        Actionkey::Forward,
        Actionkey::Backward,
        Actionkey::Left,
        Actionkey::Right,
        Actionkey::OpenInv,
        Actionkey::Sneak,
        Actionkey::Sprint,
        Actionkey::Jump,
        Actionkey::ToggleHud,
        Actionkey::ToggleDebug,
        Actionkey::ToggleChat,
    ];

    pub fn values() -> &'static [Actionkey] {
        &Self::VALUES
    }
}

impl Default for KeybindStore {
    fn default() -> Self {
        Self::new()
    }
}
