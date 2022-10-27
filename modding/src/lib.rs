use dashmap::DashMap;
use libloading::{Error, Library, Symbol};
use log::warn;
use std::any::Any;
use std::path::Path;
use std::sync::Arc;
extern crate serde;
use leafish_protocol::protocol::mapped_packet::MappedPacket;
use leafish_protocol::protocol::packet::Packet;
use leafish_protocol::protocol::{PacketType, Version};
use serde::{Deserialize, Serialize};

const CREATE_PLUGIN_FN_NAME: &[u8] = b"create_plugin";

#[no_mangle]
pub trait Plugin: Any + Send + Sync {
    fn on_enable(&self);

    fn on_disable(&self) {}

    fn get_name(&self) -> &'static str; // TODO: Should this get removed and moved into metadata?

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
            let create_plugin: Result<Symbol<fn() -> Box<dyn Plugin>>, Error> =
                unsafe { library.get(CREATE_PLUGIN_FN_NAME) };
            if let Ok(create_plugin) = create_plugin {
                let wrapped_plugin = WrappedPlugin {
                    plugin: create_plugin(),
                    _library: library,
                    meta: plugin_meta,
                    packet_handler: (None, None),
                };
                let file_name = Path::new(&path).file_name().unwrap();
                self.plugins
                    .clone()
                    .insert(String::from(file_name.to_str().unwrap()), wrapped_plugin);
                return true;
            } else {
                let file_name = Path::new(&path).file_name().unwrap();
                warn!(
                    "There is no \"create_plugin\" function available in {}!",
                    file_name.to_str().unwrap()
                );
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
    packet_handler: (
        Option<Box<dyn RawPacketHandler>>,
        Option<Box<dyn MappedPacketHandler>>,
    ),
}

#[no_mangle]
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
    pub fn description(&self) -> &String {
        &self.meta.description
    }

    #[inline]
    pub fn authors(&self) -> &Vec<String> {
        &self.meta.authors
    }

    #[inline]
    pub fn permissions(&self) -> &Vec<PluginPermission> {
        &self.meta.permissions
    }

    /// Handles an event, returns whether or not the result is `cancel`
    #[inline]
    pub fn handle_event(&self, event: ClientEvent) -> bool {
        self.plugin.handle_event(event)
    }
}

#[derive(Serialize, Deserialize, Default)]
#[repr(C)]
pub struct PluginMeta {
    /// The plugin's version
    version: u64,
    /// The plugin's description
    description: String,
    authors: Vec<String>,
    permissions: Vec<PluginPermission>, // TODO: Implement this!
    modified: bool,                     // what is this used for?
}

#[derive(Serialize, Deserialize)]
#[repr(C)]
pub enum PluginPermission {
    File,
    Network,
    Device, // grants permission for devices and drivers
    All,
}

#[no_mangle]
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

#[no_mangle]
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
    Entity(EntityEvent),
    ChannelMessage,
    Inventory(InputEvent),
    World(WorldEvent),
    Player(PlayerEvent),
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
    Keyboard(bool), // down, key board key(input)
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
    UpdateArmor,    // TODO: Should this be combined with UpdateMetadata?
    UpdateHandItem, // TODO: Should this be combined with UpdateMetadata?
    UpdateName,     // TODO: Should this be combined with UpdateMetadata?
    UpdateMetadata,
    UpdateHealth, // TODO: Should this be combined with UpdateMetadata?
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

    fn is_chunk_loaded(&self, x: i32, z: i32);
}

#[no_mangle]
pub trait ServerAccess {
    // TODO: Combine protocol_version methods into one!
    fn protocol_version_id(&self) -> i64;

    fn protocol_version(&self) -> Version;

    fn world(&self) -> Box<dyn WorldAccess>;

    fn is_connected(&self) -> bool;

    fn write_packet<T: PacketType>(&self, packet: T); // FIXME: Should this be renamed to "send_packet"?
}

#[no_mangle]
pub trait ScreenSystemAccess {
    fn pop_screen(&self) -> Box<dyn ScreenAccess>;

    fn push_screen(&self, screen_builder: ScreenBuilder);

    fn current_screen(&self) -> Box<&dyn ScreenAccess>;
}

#[no_mangle]
pub trait ScreenAccess {
    fn ty(&self) -> ScreenType;
}

#[repr(C)]
pub struct ScreenBuilder {
    name: String,
    // FIXME: Why not use a single trait instead of all these closures?
    activate: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    deactivate: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    init: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    deinit: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
}

#[no_mangle]
impl ScreenBuilder {
    #[inline]
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    #[inline]
    pub fn on_activate(
        mut self,
        on_activate: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    ) -> Self {
        self.activate = on_activate;
        self
    }

    #[inline]
    pub fn on_deactivate(
        mut self,
        on_deactivate: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    ) -> Self {
        self.deactivate = on_deactivate;
        self
    }

    #[inline]
    pub fn on_init(
        mut self,
        on_init: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    ) -> Self {
        self.init = on_init;
        self
    }

    #[inline]
    pub fn on_deinit(
        mut self,
        on_deinit: Option<dyn Fn(&ScreenSystem, Arc<render::Renderer>, &mut ui::Container)>,
    ) -> Self {
        self.deinit = on_deinit;
        self
    }
}
