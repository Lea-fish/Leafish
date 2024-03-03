use log::{info, warn};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::str::FromStr;

use winit::keyboard::KeyCode;

use crate::paths;

#[derive(Clone, Copy)]
pub struct Keybind {
    name: &'static str,
    description: &'static str,
    pub action: Actionkey,
}

pub struct KeybindStore(Mutex<HashMap<i32, Keybind>>);

impl KeybindStore {
    pub fn new() -> Self {
        let mut store = KeybindStore(Mutex::new(HashMap::new()));
        store.load_defaults();
        store.load_config();
        store
    }

    pub fn get(&self, key: KeyCode) -> Option<Keybind> {
        self.0.lock().get(&(key as i32)).copied()
    }

    pub fn set(&self, key: i32, action: Actionkey) {
        let old_key = self
            .0
            .lock()
            .iter()
            .find(|(_, v)| v.action == action)
            .expect("a action was not bound to a key?")
            .0
            .clone();

        let old_val = self.0.lock().remove(&old_key).unwrap();
        self.0.lock().insert(key, old_val);
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
                let mut store = self.0.lock();
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
                    info!("a unknown keybind was specified: {name}");
                }
            }
        }
    }

    fn save_config(&self) {
        let mut file =
            BufWriter::new(fs::File::create(paths::get_config_dir().join("keybinds.cfg")).unwrap());
        for (key, keybind) in self.0.lock().iter() {
            for line in keybind.description.lines() {
                if let Err(err) = writeln!(file, "# {}", line) {
                    warn!(
                        "couldnt write a keybind description to config file {err}, {}",
                        keybind.name
                    );
                }
            }
            if let Err(err) = write!(file, "{} {}\n\n", keybind.name, *key as i32) {
                warn!(
                    "couldnt write a keybind to config file {err}, {}",
                    keybind.name
                );
            };
        }
    }

    fn load_defaults(&self) {
        let mut s = self.0.lock();
        for bind in create_keybinds() {
            s.insert(bind.0 as i32, bind.1);
        }
    }
}

fn deserialize_key(input: &str) -> Option<i32> {
    match input.parse::<i32>() {
        Ok(num) => Some(num),
        Err(err) => {
            warn!("couldnt deserialize keybind: {err}, {input}");
            None
        }
    }
}

fn create_keybinds() -> Vec<(KeyCode, Keybind)> {
    vec![
        (
            KeyCode::KeyW,
            Keybind {
                name: "keybind_forward",
                description: "Keybinding for moving forward",
                action: Actionkey::Forward,
            },
        ),
        (
            KeyCode::KeyS,
            Keybind {
                name: "keybind_backward",
                description: "Keybinding for moving backward",
                action: Actionkey::Backward,
            },
        ),
        (
            KeyCode::KeyA,
            Keybind {
                name: "keybind_left",
                description: "Keybinding for moving to the left",
                action: Actionkey::Left,
            },
        ),
        (
            KeyCode::KeyD,
            Keybind {
                name: "keybind_right",
                description: "Keybinding for moving to the right",
                action: Actionkey::Left,
            },
        ),
        (
            KeyCode::KeyE,
            Keybind {
                name: "keybind_open_inv",
                description: "Keybinding for opening the inventory",
                action: Actionkey::OpenInv,
            },
        ),
        (
            KeyCode::ShiftLeft,
            Keybind {
                name: "keybind_sneak",
                description: "Keybinding for sneaking",
                action: Actionkey::Sneak,
            },
        ),
        (
            KeyCode::ControlLeft,
            Keybind {
                name: "keybind_sprint",
                description: "Keybinding for sprinting",
                action: Actionkey::Sprint,
            },
        ),
        (
            KeyCode::Space,
            Keybind {
                name: "keybind_jump",
                description: "Keybinding for jumping",
                action: Actionkey::Jump,
            },
        ),
        (
            KeyCode::F1,
            Keybind {
                name: "keybind_toggle_hud",
                description: "Keybinding for toggeling the hud",
                action: Actionkey::ToggleHud,
            },
        ),
        (
            KeyCode::F3,
            Keybind {
                name: "keybind_toggle_debug_info",
                description: "Keybinding for toggeling the debug info",
                action: Actionkey::ToggleDebug,
            },
        ),
        (
            KeyCode::KeyT,
            Keybind {
                name: "keybind_toggle_chat",
                description: "Keybinding for toggeling the chat",
                action: Actionkey::ToggleChat,
            },
        ),
    ]
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
