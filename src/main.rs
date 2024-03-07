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

#![recursion_limit = "300"]
#![allow(clippy::too_many_arguments)] // match standard gl functions with many arguments
#![allow(clippy::many_single_char_names)] // short variable names provide concise clarity
#![allow(clippy::float_cmp)] // float comparison used to check if changed

use arc_swap::ArcSwapOption;
use leafish_protocol::protocol::login::AccountType;
use log::{info, warn};
use shared::Version;
use ui::InterUiMessage;
use ui::UiImpl;
use ui::UiQueue;
use std::fs;
use std::sync::atomic::AtomicBool;
use winit::keyboard::Key;
extern crate leafish_shared as shared;

use structopt::StructOpt;

extern crate leafish_protocol;

pub mod ecs;
use leafish_protocol::format;
use leafish_protocol::nbt;
use leafish_protocol::protocol;
use leafish_protocol::types;
pub mod paths;
pub mod resources;
pub mod server;
pub mod settings;
pub mod ui;
pub mod world;
mod console;
pub mod accounts;
pub mod inventory;

use crate::accounts::load_accounts;
use crate::settings::*;
use leafish_protocol::protocol::login::Account;
use leafish_protocol::protocol::Error;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;

const UI_IMPL: UiImpl = UiImpl::Glow;

// TODO: Improve calculate light performance and fix capturesnapshot

pub struct Game {
    resource_manager: Arc<RwLock<resources::Manager>>,
    console: Arc<Mutex<console::Console>>,
    settings: Arc<settings::SettingStore>,
    keybinds: Arc<settings::KeybindStore>,
    should_close: AtomicBool, // FIXME: should this go into the rendering section?

    server: Arc<ArcSwapOption<server::Server>>,
    focused: AtomicBool,

    connect_error: ArcSwapOption<Error>, // FIXME: get rid of this and immediately send an UiMessage

    default_protocol_version: i32,
    current_account: Arc<Mutex<Option<Account>>>, // FIXME: use ArcSwapOption instead!
    queue: Box<dyn UiQueue>,
}

impl Game {
    pub fn send_ui_msg(&self, msg: InterUiMessage) {
        self.queue.send(msg);
    }

    pub fn connect_to(
        &self,
        address: &str,
    ) -> Result<(), Error> {
        let (protocol_version, forge_mods, fml_network_version) =
            match protocol::Conn::new(address, self.default_protocol_version)
                .and_then(|conn| conn.do_status())
            {
                Ok(res) => {
                    info!(
                        "Detected server protocol version {}",
                        res.0.version.protocol
                    );
                    (
                        res.0.version.protocol,
                        res.0.forge_mods,
                        res.0.fml_network_version,
                    )
                }
                Err(err) => {
                    warn!(
                        "Error pinging server {} to get protocol version: {:?}, defaulting to {}",
                        address, err, self.default_protocol_version
                    );
                    (self.default_protocol_version, vec![], None)
                }
            };
        if !Version::from_id(protocol_version as u32).is_supported() {
            return Err(Error::Err(format!(
                "The server's version isn't supported!\n(protocol version: {})",
                protocol_version
            )));
        }
        let address = address.to_owned();
        let resources = self.resource_manager.clone();
        let account = self.current_account.clone();
        let result = thread::spawn(move || {
            server::Server::connect(
                resources,
                account.lock().as_ref().unwrap(),
                &address,
                protocol_version,
                forge_mods,
                fml_network_version,
            )
        })
        .join();
        match result {
            Ok(result) => {
                match result {
                    Ok(srv) => {
                        self.server.store(Some(srv));
                        Ok(())
                    }
                    Err(err) => {
                        let str = err.to_string();
                        self.connect_error.store(Some(Arc::new(err)));
                        // self.server.disconnect_reason = Some(Component::from_string(&*err.to_string()));
                        Err(Error::Err(str))
                    }
                }
            }
            Err(_) => Err(Error::Err("Unknown".to_string())),
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "leafish")]
struct Opt {
    /// Log decoded packets received from network
    #[structopt(short = "n", long = "network-debug")]
    network_debug: bool,

    /// Parse a network packet from a file
    #[structopt(short = "N", long = "network-parse-packet")]
    network_parse_packet: Option<String>,

    /// Protocol version to use in the autodetection ping
    #[structopt(short = "p", long = "default-protocol-version")]
    default_protocol_version: Option<String>,
    #[structopt(long)]
    uuid: Option<String>,
    #[structopt(long)]
    name: Option<String>,
    #[structopt(long)]
    token: Option<String>,
}

// TODO: Hide own character and show only the right hand. (with an item)
// TODO: Simplify error messages in server list.
// TODO: Render skin of players joining after one self.
// TODO: Implement arm swing animation!
// TODO: Implement attacking entities!
// TODO: Fix cursor grabbing/visibility/transparency of window.
// TODO: Improve clouds.
// TODO: Fix pistons.
fn main() {
    let opt = Opt::from_args();
    #[allow(clippy::arc_with_non_send_sync)]
    let con = Arc::new(Mutex::new(console::Console::new()));
    let proxy = console::ConsoleProxy::new(con.clone());

    log::set_boxed_logger(Box::new(proxy)).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    info!("Starting Leafish...");

    let settings = Arc::new(SettingStore::new());
    let keybinds = Arc::new(KeybindStore::new());
    info!("settings all loaded!");

    con.lock().configure(&settings);

    let res = resources::Manager::new();
    let resource_manager = Arc::new(RwLock::new(res));

    let mut accounts = load_accounts().unwrap_or_default();
    if let Some((name, uuid, token)) = opt
        .name
        .clone()
        .and_then(|name| {
            opt.uuid
                .clone()
                .map(|uuid| opt.token.clone().map(|token| (name, uuid, token)))
        })
        .flatten()
    {
        println!("Got microsoft credentials, adding account...");
        accounts.push(Account {
            name: name.clone(),
            uuid: Some(uuid),
            verification_tokens: vec![name, "".to_string(), token],
            head_img_data: None,
            account_type: AccountType::Microsoft,
        });
    }
    /*screen_sys.add_screen(Box::new(screen::launcher::Launcher::new(
        Arc::new(Mutex::new(accounts)),
        screen_sys.clone(),
        active_account.clone(),
    )));*/

    let default_protocol_version = protocol::versions::protocol_name_to_protocol_version(
        opt.default_protocol_version.unwrap_or_default(),
    );

    let game = Arc::new(Game {
        server: Arc::new(ArcSwapOption::new(None)),
        focused: AtomicBool::new(false),
        resource_manager: resource_manager.clone(),
        console: con,
        should_close: AtomicBool::new(false),
        connect_error: ArcSwapOption::new(None),
        default_protocol_version,
        current_account: Arc::new(Mutex::new(None)),
        settings,
        keybinds,
        queue: ui::ui_queue(UI_IMPL),
    });
    if opt.network_debug {
        protocol::enable_network_debug();
    }

    if let Some(filename) = opt.network_parse_packet {
        let data = fs::read(filename).unwrap();
        protocol::try_parse_packet(data, default_protocol_version);
        return;
    }

    ui::start_ui(&game, UI_IMPL).unwrap();
}

pub const DEBUG: bool = false;

pub trait KeyCmp {
    fn eq_ignore_case(&self, other: char) -> bool;
}

impl KeyCmp for Key {
    fn eq_ignore_case(&self, other: char) -> bool {
        match self {
            Key::Character(content) => {
                if content.as_str().len() != 1 {
                    return false;
                }
                let chr = content.as_str().chars().next().unwrap();
                if !other.is_alphabetic() {
                    return chr == other;
                }
                chr.to_ascii_lowercase() == other || chr.to_ascii_uppercase() == other
            }
            _ => false,
        }
    }
}
