use std::any::Any;
use libloading::{Library, Symbol, Error};
use std::sync::Arc;
use dashmap::DashMap;
use std::path::Path;
use log::warn;
extern crate serde;
use serde::{Deserialize, Serialize};
use leafish_protocol::protocol::mapped_packet::MappedPacket;
use leafish_protocol::protocol::packet::Packet;
use leafish_protocol::protocol::Version;

pub trait Plugin: Any + Send + Sync {

    fn on_enable(&self);

    fn on_disable(&self) {}

    fn get_name(&self) -> &'static str;

    fn handle_event(&self, event: ClientEvent) -> bool;

}

pub struct PluginManager {

    plugins: Arc<DashMap<String, WrappedPlugin>>,

}

impl PluginManager {

    pub fn load_plugin(&self, path: String) -> bool {
        let raw_plugin_meta = std::fs::read_to_string(format!("{}/meta.json", path));
        let plugin_meta = if let Ok(meta) = raw_plugin_meta {
            if let Ok(meta) = serde_json::from_str(&*meta) {
                meta
            } else {
                PluginMeta::default()
            }
        } else {
            PluginMeta::default()
        };

        let potential_library = unsafe { Library::new(format!("{}/bin", path)) };
        if let Ok(library) = potential_library {
            let create_plugin: Result<Symbol<fn() -> Box<dyn Plugin>>, Error> = unsafe { library.get(b"create_plugin") };
            if let Ok(create_plugin) = create_plugin {
                let wrapped_plugin = WrappedPlugin {
                    plugin: create_plugin(),
                    _library: library,
                    meta: plugin_meta,
                    packet_handler: (None, None),
                };
                let file_name = Path::new(&path).file_name().unwrap();
                self.plugins.clone().insert(String::from(file_name.to_str().unwrap()), wrapped_plugin);
                return true;
            } else {
                let file_name = Path::new(&path).file_name().unwrap();
                warn!("There is no \"create_plugin\" function available in {}!", file_name.to_str().unwrap());
            }
        }
        false
    }

    pub fn unload_plugin(&self, plugin: String) -> bool {
        if let Some(plugin) = self.plugins.clone().remove(&*plugin) {
            plugin.1.plugin.on_disable();
            return true;
        }
        false
    }

    pub fn unload_all(&self) {
        for plugin in self.plugins.clone().iter() {
            plugin.plugin.on_disable();
        }
        self.plugins.clear();
    }

}

pub struct WrappedPlugin {

    plugin: Box<dyn Plugin>,
    _library: Library,
    meta: PluginMeta,
    packet_handler: (Option<Box<dyn RawPacketHandler>>, Option<Box<dyn MappedPacketHandler>>),

}

impl WrappedPlugin {

    #[inline]
    pub fn name(&self) -> &'static str {
        self.plugin.get_name()
    }

    #[inline]
    pub fn version(&self) -> u64 {
        self.meta.version
    }

    #[inline]
    pub fn authors(&self) -> &Vec<String> {
        &self.meta.authors
    }

    #[inline]
    pub fn permissions(&self) -> &Vec<PluginPermission> {
        &self.meta.permissions
    }

    /// handles an event, returns whether or not the result is `cancel`
    #[inline]
    pub fn handle_event(&self, event: ClientEvent) -> bool {
        self.plugin.handle_event(event)
    }

}

#[derive(Serialize, Deserialize, Default)]
#[repr(C)]
pub struct PluginMeta {

    version: u64, /// The plugin's version.
    authors: Vec<String>,
    permissions: Vec<PluginPermission>, // TODO: Implement this!
    modified: bool, // what is this used for?

}


#[derive(Serialize, Deserialize)]
#[repr(C)]
pub enum PluginPermission {

    File,
    Network,
    Device, // grants permission for devices and drivers
    All,

}

pub trait RawPacketHandler {

    #[inline]
    fn incoming_priority(&self) -> i64 {
        0
    }

    fn handle_incoming(&self, packet: *mut Packet) -> bool;

    #[inline]
    fn outgoing_priority(&self) -> i64 {
        0
    }

    fn handle_outgoing(&self, packet: *mut Packet) -> bool;

}

pub trait MappedPacketHandler {

    #[inline]
    fn incoming_priority(&self) -> i64 {
        0
    }

    fn handle_incoming(&self, packet: *mut MappedPacket) -> bool;

    #[inline]
    fn outgoing_priority(&self) -> i64 {
        0
    }

    fn handle_outgoing(&self, packet: *mut MappedPacket) -> bool;

}

#[repr(C)]
pub enum ClientEvent {

    UserInput(InputEvent),
    Screen(ScreenEvent),
    Entity,
    ChannelMessage,
    Inventory(InputEvent),
    World(WorldEvent),
    Player,

}

/// Events for local player updates
#[repr(C)]
pub enum PlayerEvent {

    Exp,
    Health,
    Position,
    Vehicle,
    Block,
    Interact,
    EntityInteract,
    HotBar,
    Food,

}

#[repr(C)]
pub enum InputEvent {

    Mouse(MouseAction),
    Keyboard(bool, ), // down, key board key(input)

}

#[repr(C)]
pub enum WorldEvent {

    Block,
    Weather,
    Time,

}

#[repr(C)]
pub enum InventoryEvent {

    Open,
    Close,
    UpdateSlot,

}

#[repr(C)]
pub enum ScreenEvent {

    Open,
    Close,

}

#[repr(C)]
pub enum EntityEvent {

    Spawn,
    Despawn,
    Move,
    UpdateArmor,
    UpdateHandItem,
    UpdateName,

}

#[repr(C)]
pub enum MouseAction {

    UpdateKey(bool, MouseKey), // down, mouse key(input)
    UseWheel(f64),

}

#[repr(C)]
pub enum MouseKey {

    Left,
    Middle, // pressed mouse wheel
    Right,
    Other(i64),

}

#[no_mangle]
pub trait WorldAccess {

    fn get_block(&self);

    fn set_block(&self);

}

#[no_mangle]
pub trait ServerAccess {

    fn protocol(&self) -> i64;

    fn mapped_version(&self) -> Version;

    fn world(&self) -> Box<dyn WorldAccess>;

}

#[no_mangle]
pub trait ScreenSystemAccess {

    fn pop_screen(&self);

    fn push_screen(&self);

}

#[repr(C)]
pub struct ScreenBuilder {

    name: String,
    active: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    de_active: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    init: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    de_init: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,

}