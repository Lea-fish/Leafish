// Copyright 2015 Matthew Collins
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

use crate::entity;
use crate::format;
use crate::inventory::material::versions::to_material;
use crate::inventory::{Inventory, InventoryContext, Item};
use crate::protocol::{self, forge, mojang, packet};
use crate::render;
use crate::render::hud::HudContext;
use crate::render::Renderer;
use crate::resources;
use crate::screen::respawn::Respawn;
use crate::screen::ScreenSystem;
use crate::settings::vars::Actionkey;
use crate::shared::{Axis, Position};
use crate::types::hash::FNVHash;
use crate::types::GameMode;
use crate::world;
use crate::world::{block, CPos, LightData, LightUpdate};
use crate::{ecs, Game};
use cgmath::prelude::*;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use instant::{Duration, Instant};
use leafish_protocol::format::{Component, TextComponent};
use leafish_protocol::protocol::packet::play::serverbound::{
    ClientSettings, ClientSettings_u8_Handsfree,
};
use leafish_protocol::protocol::packet::{DigType, Packet};
use leafish_protocol::protocol::{Conn, VarInt, Version};
use leafish_shared::direction::Direction as BlockDirection;
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use parking_lot::RwLock;
use rand::{self, Rng};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod plugin_messages;
mod sun;
pub mod target;

#[derive(Default)]
pub struct DisconnectData {
    pub disconnect_reason: Option<format::Component>,
    just_disconnected: bool,
}

struct WorldData {
    world_age: i64,         // move to world?
    world_time: f64,        // move to world?
    world_time_target: f64, // move to world?
    tick_time: bool,        // move to world?
}

impl Default for WorldData {
    fn default() -> Self {
        WorldData {
            world_age: 0,
            world_time: 0.0,
            world_time_target: 0.0,
            tick_time: true,
        }
    }
}

pub struct Server {
    uuid: protocol::UUID,
    conn: Arc<RwLock<Option<protocol::Conn>>>,
    pub protocol_version: i32,
    mapped_protocol_version: Version,
    forge_mods: Vec<forge::ForgeMod>,
    pub disconnect_data: Arc<RwLock<DisconnectData>>,

    pub world: Arc<world::World>,
    pub entities: Arc<RwLock<ecs::Manager>>,
    world_data: Arc<RwLock<WorldData>>,

    resources: Arc<RwLock<resources::Manager>>,
    version: RwLock<usize>,

    // Entity accessors
    game_info: ecs::Key<entity::GameInfo>,
    player_movement: ecs::Key<entity::player::PlayerMovement>,
    gravity: ecs::Key<entity::Gravity>,
    position: ecs::Key<entity::Position>,
    target_position: ecs::Key<entity::TargetPosition>,
    velocity: ecs::Key<entity::Velocity>,
    gamemode: ecs::Key<GameMode>,
    pub rotation: ecs::Key<entity::Rotation>,
    target_rotation: ecs::Key<entity::TargetRotation>,
    block_break_info: Mutex<BlockBreakInfo>,
    //
    pub player: Arc<RwLock<Option<ecs::Entity>>>,
    entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
    players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,

    tick_timer: RwLock<f64>,
    entity_tick_timer: RwLock<f64>,
    pub received_chat_at: Arc<RwLock<Option<Instant>>>,

    sun_model: RwLock<Option<sun::SunModel>>,
    target_info: Arc<RwLock<target::Info>>,
    pub render_list_computer: Sender<bool>,
    pub render_list_computer_notify: Receiver<bool>,
    pub hud_context: Arc<RwLock<HudContext>>,
    pub inventory_context: Arc<RwLock<InventoryContext>>,
    fps: RwLock<u32>,
    fps_start: RwLock<u128>,
    pub dead: RwLock<bool>,
    just_died: RwLock<bool>,
    close_death_screen: RwLock<bool>,
}

#[derive(Debug)]
pub struct BlockBreakInfo {
    break_position: Position,
    break_face: BlockDirection,
    hardness: f32,
    progress: f32,
    delay: u8,
    active: bool,
}

#[derive(Debug)]
pub struct PlayerInfo {
    name: String,
    uuid: protocol::UUID,
    skin_url: Option<String>,

    display_name: Option<format::Component>,
    ping: i32,
    gamemode: GameMode,
}

impl Server {
    pub fn connect(
        resources: Arc<RwLock<resources::Manager>>,
        profile: mojang::Profile,
        address: &str,
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        fml_network_version: Option<i64>,
        renderer: Arc<RwLock<Renderer>>,
        hud_context: Arc<RwLock<HudContext>>,
    ) -> Result<Arc<Server>, protocol::Error> {
        let mut conn = protocol::Conn::new(address, protocol_version)?;

        let tag = match fml_network_version {
            Some(1) => "\0FML\0",
            Some(2) => "\0FML2\0",
            None => "",
            _ => panic!("unsupported FML network version: {:?}", fml_network_version),
        };

        let host = conn.host.clone() + tag;
        let port = conn.port;
        conn.write_packet(protocol::packet::handshake::serverbound::Handshake {
            protocol_version: protocol::VarInt(protocol_version),
            host,
            port,
            next: protocol::VarInt(2),
        })?;
        conn.state = protocol::State::Login;
        conn.write_packet(protocol::packet::login::serverbound::LoginStart {
            username: profile.username.clone(),
        })?;

        use std::rc::Rc;
        let (server_id, public_key, verify_token);
        loop {
            match conn.read_packet()? {
                protocol::packet::Packet::SetInitialCompression(val) => {
                    conn.set_compression(val.threshold.0);
                }
                protocol::packet::Packet::EncryptionRequest(val) => {
                    server_id = Rc::new(val.server_id);
                    public_key = Rc::new(val.public_key.data);
                    verify_token = Rc::new(val.verify_token.data);
                    break;
                }
                protocol::packet::Packet::EncryptionRequest_i16(val) => {
                    server_id = Rc::new(val.server_id);
                    public_key = Rc::new(val.public_key.data);
                    verify_token = Rc::new(val.verify_token.data);
                    break;
                }
                protocol::packet::Packet::LoginSuccess_String(val) => {
                    warn!("Server is running in offline mode");
                    debug!("Login: {} {}", val.username, val.uuid);
                    conn.state = protocol::State::Play;
                    let uuid = protocol::UUID::from_str(&val.uuid).unwrap();
                    let server = Server::connect0(
                        conn,
                        protocol_version,
                        forge_mods,
                        uuid,
                        resources,
                        renderer,
                        hud_context,
                    );
                    return Ok(server);
                }
                protocol::packet::Packet::LoginSuccess_UUID(val) => {
                    warn!("Server is running in offline mode");
                    debug!("Login: {} {:?}", val.username, val.uuid);
                    conn.state = protocol::State::Play;
                    let server = Server::connect0(
                        conn,
                        protocol_version,
                        forge_mods,
                        val.uuid,
                        resources,
                        renderer,
                        hud_context,
                    );

                    return Ok(server);
                }
                protocol::packet::Packet::LoginDisconnect(val) => {
                    return Err(protocol::Error::Disconnect(val.reason))
                }
                val => return Err(protocol::Error::Err(format!("Wrong packet 1: {:?}", val))),
            };
        }

        let mut shared = [0; 16];
        rand::thread_rng().fill(&mut shared);

        let shared_e = rsa_public_encrypt_pkcs1::encrypt(&public_key, &shared).unwrap();
        let token_e = rsa_public_encrypt_pkcs1::encrypt(&public_key, &verify_token).unwrap();

        profile.join_server(&server_id, &shared, &public_key)?;

        if protocol_version >= 47 {
            conn.write_packet(protocol::packet::login::serverbound::EncryptionResponse {
                shared_secret: protocol::LenPrefixedBytes::new(shared_e),
                verify_token: protocol::LenPrefixedBytes::new(token_e),
            })?;
        } else {
            conn.write_packet(
                protocol::packet::login::serverbound::EncryptionResponse_i16 {
                    shared_secret: protocol::LenPrefixedBytes::new(shared_e),
                    verify_token: protocol::LenPrefixedBytes::new(token_e),
                },
            )?;
        }

        conn.enable_encyption(&shared);

        let uuid;
        let compression_threshold = conn.compression_threshold;
        loop {
            match conn.read_packet()? {
                protocol::packet::Packet::SetInitialCompression(val) => {
                    conn.set_compression(val.threshold.0);
                }
                protocol::packet::Packet::LoginSuccess_String(val) => {
                    debug!("Login: {} {}", val.username, val.uuid);
                    uuid = protocol::UUID::from_str(&val.uuid).unwrap();
                    conn.state = protocol::State::Play;
                    break;
                }
                protocol::packet::Packet::LoginSuccess_UUID(val) => {
                    debug!("Login: {} {:?}", val.username, val.uuid);
                    uuid = val.uuid;
                    conn.state = protocol::State::Play;
                    break;
                }
                protocol::packet::Packet::LoginDisconnect(val) => {
                    return Err(protocol::Error::Disconnect(val.reason))
                }
                protocol::packet::Packet::LoginPluginRequest(req) => {
                    match req.channel.as_ref() {
                        "fml:loginwrapper" => {
                            let mut cursor = std::io::Cursor::new(req.data);
                            let channel: String = protocol::Serializable::read_from(&mut cursor)?;

                            let (id, mut data) = protocol::Conn::read_raw_packet_from(
                                &mut cursor,
                                compression_threshold,
                            )?;

                            match channel.as_ref() {
                                "fml:handshake" => {
                                    let packet =
                                        forge::fml2::FmlHandshake::packet_by_id(id, &mut data)?;
                                    use forge::fml2::FmlHandshake::*;
                                    match packet {
                                        ModList {
                                            mod_names,
                                            channels,
                                            registries,
                                        } => {
                                            info!("ModList mod_names={:?} channels={:?} registries={:?}", mod_names, channels, registries);
                                            conn.write_fml2_handshake_plugin_message(
                                                req.message_id,
                                                Some(&ModListReply {
                                                    mod_names,
                                                    channels,
                                                    registries,
                                                }),
                                            )?;
                                        }
                                        ServerRegistry {
                                            name,
                                            snapshot_present: _,
                                            snapshot: _,
                                        } => {
                                            info!("ServerRegistry {:?}", name);
                                            conn.write_fml2_handshake_plugin_message(
                                                req.message_id,
                                                Some(&Acknowledgement),
                                            )?;
                                        }
                                        ConfigurationData { filename, contents } => {
                                            info!(
                                                "ConfigurationData filename={:?} contents={}",
                                                filename,
                                                String::from_utf8_lossy(&contents)
                                            );
                                            conn.write_fml2_handshake_plugin_message(
                                                req.message_id,
                                                Some(&Acknowledgement),
                                            )?;
                                        }
                                        _ => unimplemented!(),
                                    }
                                }
                                _ => panic!(
                                    "unknown LoginPluginRequest fml:loginwrapper channel: {:?}",
                                    channel
                                ),
                            }
                        }
                        _ => panic!("unsupported LoginPluginRequest channel: {:?}", req.channel),
                    }
                }
                val => return Err(protocol::Error::Err(format!("Wrong packet 2: {:?}", val))),
            }
        }

        let server = Server::connect0(
            conn,
            protocol_version,
            forge_mods,
            uuid,
            resources,
            renderer,
            hud_context,
        );

        Ok(server)
    }

    fn connect0(
        conn: Conn,
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        uuid: protocol::UUID,
        resources: Arc<RwLock<resources::Manager>>,
        renderer: Arc<RwLock<Renderer>>,
        hud_context: Arc<RwLock<HudContext>>,
    ) -> Arc<Server> {
        let server_callback = Arc::new(Mutex::new(None));
        let inner_server = server_callback.clone();
        let mut inner_server = inner_server.lock();
        Self::spawn_reader(conn.clone(), server_callback.clone());
        let light_updater = Self::spawn_light_updater(server_callback.clone());
        let render_list_computer =
            Self::spawn_render_list_computer(server_callback, renderer.clone());
        let conn = Arc::new(RwLock::new(Some(conn)));
        let server = Arc::new(Server::new(
            protocol_version,
            forge_mods,
            uuid,
            resources,
            conn,
            light_updater,
            render_list_computer.0.clone(),
            render_list_computer.1,
            hud_context,
            &renderer.read(),
        ));
        server.hud_context.clone().write().server = Some(server.clone());

        let actual_server = server.clone();
        inner_server.replace(actual_server);
        render_list_computer.0.send(true).unwrap();
        server
    }

    fn spawn_reader(mut read: protocol::Conn, server: Arc<Mutex<Option<Arc<Server>>>>) {
        thread::spawn(move || loop {
            let server = server.clone().lock().as_ref().unwrap().clone();
            let pck = read.read_packet();
            match pck {
                Ok(pck) => match pck {
                    Packet::KeepAliveClientbound_i64(keep_alive) => {
                        server.on_keep_alive_i64(keep_alive);
                    }
                    Packet::KeepAliveClientbound_VarInt(keep_alive) => {
                        server.on_keep_alive_varint(keep_alive);
                    }
                    Packet::KeepAliveClientbound_i32(keep_alive) => {
                        server.on_keep_alive_i32(keep_alive);
                    }
                    Packet::ChunkData_NoEntities(chunk_data) => {
                        server.on_chunk_data_no_entities(chunk_data);
                    }
                    Packet::ChunkData_NoEntities_u16(chunk_data) => {
                        server.on_chunk_data_no_entities_u16(chunk_data);
                    }
                    Packet::ChunkData_17(chunk_data) => {
                        server.on_chunk_data_17(chunk_data);
                    }
                    Packet::ChunkDataBulk(bulk) => {
                        server.on_chunk_data_bulk(bulk);
                    }
                    Packet::ChunkDataBulk_17(bulk) => {
                        server.on_chunk_data_bulk_17(bulk);
                    }
                    Packet::BlockChange_VarInt(block_change) => {
                        server.on_block_change_varint(block_change);
                    }
                    Packet::BlockChange_u8(block_change) => {
                        server.on_block_change_u8(block_change);
                    }
                    Packet::MultiBlockChange_Packed(block_change) => {
                        server.on_multi_block_change_packed(block_change);
                    }
                    Packet::MultiBlockChange_VarInt(block_change) => {
                        server.on_multi_block_change_varint(block_change);
                    }
                    Packet::MultiBlockChange_u16(block_change) => {
                        server.on_multi_block_change_u16(block_change);
                    }
                    Packet::UpdateBlockEntity(block_update) => {
                        server.on_block_entity_update(block_update);
                    }
                    Packet::ChunkData_Biomes3D(chunk_data) => {
                        server.on_chunk_data_biomes3d(chunk_data);
                    }
                    Packet::ChunkData_Biomes3D_VarInt(chunk_data) => {
                        server.on_chunk_data_biomes3d_varint(chunk_data);
                    }
                    Packet::ChunkData_Biomes3D_bool(chunk_data) => {
                        server.on_chunk_data_biomes3d_bool(chunk_data);
                    }
                    Packet::ChunkData(chunk_data) => {
                        server.on_chunk_data(chunk_data);
                    }
                    Packet::ChunkData_HeightMap(chunk_data) => {
                        server.on_chunk_data_heightmap(chunk_data);
                    }
                    Packet::UpdateSign(update_sign) => {
                        server.on_sign_update(update_sign);
                    }
                    Packet::UpdateSign_u16(update_sign) => {
                        server.on_sign_update_u16(update_sign);
                    }
                    Packet::UpdateBlockEntity_Data(block_update) => {
                        server.on_block_entity_update_data(block_update);
                    }
                    Packet::ChunkUnload(chunk_unload) => {
                        server.on_chunk_unload(chunk_unload);
                    }
                    Packet::EntityDestroy(entity_destroy) => {
                        server.on_entity_destroy(entity_destroy);
                    }
                    Packet::EntityDestroy_u8(entity_destroy) => {
                        server.on_entity_destroy_u8(entity_destroy);
                    }
                    Packet::EntityMove_i8_i32_NoGround(m) => {
                        server.on_entity_move_i8_i32_noground(m);
                    }
                    Packet::EntityMove_i8(m) => {
                        server.on_entity_move_i8(m);
                    }
                    Packet::EntityMove_i16(m) => {
                        server.on_entity_move_i16(m);
                    }
                    Packet::EntityLook_VarInt(look) => {
                        server.on_entity_look_varint(look);
                    }
                    Packet::EntityLook_i32_NoGround(look) => {
                        server.on_entity_look_i32_noground(look);
                    }
                    Packet::JoinGame_HashedSeed_Respawn(join) => {
                        server.on_game_join_hashedseed_respawn(join);
                    }
                    Packet::JoinGame_i8(join) => {
                        server.on_game_join_i8(join);
                    }
                    Packet::JoinGame_i8_NoDebug(join) => {
                        server.on_game_join_i8_nodebug(join);
                    }
                    Packet::JoinGame_i32(join) => {
                        server.on_game_join_i32(join);
                    }
                    Packet::JoinGame_i32_ViewDistance(join) => {
                        server.on_game_join_i32_viewdistance(join);
                    }
                    Packet::JoinGame_WorldNames(join) => {
                        server.on_game_join_worldnames(join);
                    }
                    Packet::JoinGame_WorldNames_IsHard(join) => {
                        server.on_game_join_worldnames_ishard(join);
                    }
                    Packet::TeleportPlayer_WithConfirm(teleport) => {
                        server.on_teleport_player_withconfirm(teleport);
                    }
                    Packet::TeleportPlayer_NoConfirm(teleport) => {
                        server.on_teleport_player_noconfirm(teleport);
                    }
                    Packet::TeleportPlayer_OnGround(teleport) => {
                        server.on_teleport_player_onground(teleport);
                    }
                    Packet::Respawn_Gamemode(respawn) => {
                        server.on_respawn_gamemode(respawn);
                    }
                    Packet::Respawn_HashedSeed(respawn) => {
                        server.on_respawn_hashedseed(respawn);
                    }
                    Packet::Respawn_NBT(respawn) => {
                        server.on_respawn_nbt(respawn);
                    }
                    Packet::Respawn_WorldName(respawn) => {
                        server.on_respawn_worldname(respawn);
                    }
                    Packet::EntityTeleport_f64(entity_teleport) => {
                        server.on_entity_teleport_f64(entity_teleport);
                    }
                    Packet::EntityTeleport_i32(entity_teleport) => {
                        server.on_entity_teleport_i32(entity_teleport);
                    }
                    Packet::EntityTeleport_i32_i32_NoGround(entity_teleport) => {
                        server.on_entity_teleport_i32_i32_noground(entity_teleport);
                    }
                    Packet::EntityLookAndMove_i8_i32_NoGround(lookmove) => {
                        server.on_entity_look_and_move_i8_i32_noground(lookmove);
                    }
                    Packet::EntityLookAndMove_i8(lookmove) => {
                        server.on_entity_look_and_move_i8(lookmove);
                    }
                    Packet::EntityLookAndMove_i16(lookmove) => {
                        server.on_entity_look_and_move_i16(lookmove);
                    }
                    Packet::SpawnPlayer_i32_HeldItem_String(spawn) => {
                        server.on_player_spawn_i32_helditem_string(spawn);
                    }
                    Packet::SpawnPlayer_i32_HeldItem(spawn) => {
                        server.on_player_spawn_i32_helditem(spawn);
                    }
                    Packet::SpawnPlayer_i32(spawn) => {
                        server.on_player_spawn_i32(spawn);
                    }
                    Packet::SpawnPlayer_f64(spawn) => {
                        server.on_player_spawn_f64(spawn);
                    }
                    Packet::SpawnPlayer_f64_NoMeta(spawn) => {
                        server.on_player_spawn_f64_nometa(spawn);
                    }
                    Packet::PlayerInfo(player_info) => {
                        server.on_player_info(player_info);
                    }
                    Packet::ConfirmTransaction(transaction) => {
                        read.write_packet(
                            packet::play::serverbound::ConfirmTransactionServerbound {
                                id: 0, // TODO: Use current container id, if the id of the transaction is not 0.
                                action_number: transaction.action_number,
                                accepted: true,
                            },
                        )
                        .unwrap();
                    }
                    Packet::UpdateLight_NoTrust(update_light) => {
                        server.world.clone().lighting_cache.clone().write().insert(
                            CPos(update_light.chunk_x.0, update_light.chunk_z.0),
                            LightData {
                                arrays: Cursor::new(update_light.light_arrays),
                                block_light_mask: update_light.block_light_mask.0,
                                sky_light_mask: update_light.sky_light_mask.0,
                            },
                        );
                    }
                    Packet::UpdateLight_WithTrust(update_light) => {
                        // TODO: Add specific stuff!
                        server.world.clone().lighting_cache.clone().write().insert(
                            CPos(update_light.chunk_x.0, update_light.chunk_z.0),
                            LightData {
                                arrays: Cursor::new(update_light.light_arrays),
                                block_light_mask: update_light.block_light_mask.0,
                                sky_light_mask: update_light.sky_light_mask.0,
                            },
                        );
                    }
                    Packet::ChangeGameState(game_state) => {
                        server.on_game_state_change(game_state);
                    }
                    Packet::UpdateHealth(update_health) => {
                        server.on_update_health(
                            update_health.health,
                            update_health.food.0 as u8,
                            update_health.food_saturation as u8,
                        );
                    }
                    Packet::UpdateHealth_u16(update_health) => {
                        server.on_update_health(
                            update_health.health,
                            update_health.food as u8,
                            update_health.food_saturation as u8,
                        );
                    }
                    Packet::TimeUpdate(time_update) => {
                        server.on_time_update(time_update);
                    }
                    Packet::Disconnect(disconnect) => {
                        server.disconnect(Some(disconnect.reason));
                    }
                    Packet::ServerMessage_NoPosition(server_message) => {
                        server.on_servermessage_noposition(server_message);
                    }
                    Packet::ServerMessage_Position(server_message) => {
                        server.on_servermessage_position(server_message);
                    }
                    Packet::ServerMessage_Sender(server_message) => {
                        server.on_servermessage_sender(server_message);
                    }
                    Packet::PlayerInfo_String(player_info) => {
                        server.on_player_info_string(player_info);
                    }
                    Packet::PluginMessageClientbound_i16(plugin_message) => {
                        server.on_plugin_message_clientbound_i16(plugin_message);
                    }
                    Packet::PluginMessageClientbound(plugin_message) => {
                        server.on_plugin_message_clientbound_1(plugin_message);
                    }
                    Packet::SetExperience(set_exp) => {
                        server
                            .hud_context
                            .clone()
                            .write()
                            .update_exp(set_exp.experience_bar, set_exp.level.0);
                    }
                    Packet::SetCurrentHotbarSlot(set_slot) => {
                        if set_slot.slot <= 8 {
                            server.inventory_context.clone().write().hotbar_index = set_slot.slot;
                            server
                                .hud_context
                                .clone()
                                .write()
                                .update_slot_index(set_slot.slot);
                        } else {
                            warn!("The server tried to set the hotbar slot to {}, although it has to be in a range of 0-8! Did it try to crash you?", set_slot.slot);
                        }
                    }
                    Packet::WindowItems(_window_items) => {
                        debug!("items!");
                    }
                    Packet::WindowSetSlot(set_slot) => {
                        let inventory = server.inventory_context.clone();
                        let mut top_inventory = inventory.write();
                        let inventory = if let Some(inventory) = top_inventory.inventory.as_ref() {
                            inventory.clone()
                        } else {
                            top_inventory.player_inventory.clone()
                        };
                        let curr_slots = inventory.clone().read().size();
                        if set_slot.slot < 0 || set_slot.slot >= curr_slots {
                            if set_slot.slot == -1 {
                                let item = set_slot.item.map(|stack| {
                                    let id = stack.id;
                                    Item {
                                        stack,
                                        material: to_material(
                                            id as u16,
                                            server.mapped_protocol_version,
                                        ),
                                    }
                                });
                                top_inventory.cursor = item; // TODO: Set to HUD and make it dirty!
                            } else {
                                warn!("The server tried to set an item to slot {} but the current inventory only has {} slots. Did it try to crash you?", set_slot.id + 1, curr_slots);
                            }
                        } else {
                            debug!(
                                "set item to {}, {}, {}",
                                set_slot.id,
                                set_slot.slot,
                                set_slot.item.as_ref().map_or(0, |s| s.id)
                            );
                            let item = set_slot.item.map(|stack| {
                                let id = stack.id;
                                Item {
                                    stack,
                                    material: to_material(
                                        id as u16,
                                        server.mapped_protocol_version,
                                    ),
                                }
                            });
                            inventory.clone().write().set_item(set_slot.slot, item);
                        }
                    }
                    _ => {
                        // debug!("other packet!");
                    }
                },
                Err(err) => {
                    if server
                        .disconnect_data
                        .clone()
                        .read()
                        .disconnect_reason
                        .is_none()
                    {
                        server
                            .disconnect_data
                            .clone()
                            .write()
                            .disconnect_reason
                            .replace(Component::Text(TextComponent::new(&*format!(
                                "An error occurred while reading a packet: {}",
                                err
                            ))));
                    }
                }
            }
        });
    }

    fn spawn_light_updater(_server: Arc<Mutex<Option<Arc<Server>>>>) -> Sender<LightUpdate> {
        let (tx, rx) = unbounded();
        thread::spawn(move || loop {
            /*
            let server = server.clone().lock().as_ref().unwrap().clone();
            let mut done = false; // TODO: Improve performance!
            while !done {
                let start = Instant::now();
                let mut updates_performed = 0;
                let world_cloned = server.world.clone();
                let mut interrupt = false;
                while let Ok(update) = rx.try_recv() {
                    updates_performed += 1;
                    world_cloned.do_light_update(update);
                    if (updates_performed & 0xFFF == 0) && start.elapsed().subsec_nanos() >= 5000000 {
                        // 5 ms for light updates
                        interrupt = true;
                        break;
                    }
                }
                if !interrupt {
                    done = true;
                }
                thread::sleep(Duration::from_millis(1));
            }*/
            while let Ok(_update) = rx.try_recv() {}
            thread::sleep(Duration::from_millis(1000));
        });
        tx
    }

    fn spawn_render_list_computer(
        server: Arc<Mutex<Option<Arc<Server>>>>,
        renderer: Arc<RwLock<Renderer>>,
    ) -> (Sender<bool>, Receiver<bool>) {
        let (tx, rx) = unbounded();
        let (etx, erx) = unbounded();
        thread::spawn(move || loop {
            let _: bool = rx.recv().unwrap();
            let server = server.clone().lock().as_ref().unwrap().clone();
            let world = server.world.clone();
            world.compute_render_list(renderer.clone());
            while rx.try_recv().is_ok() {}
            etx.send(true).unwrap();
        });
        (tx, erx)
    }

    pub fn dummy_server(
        resources: Arc<RwLock<resources::Manager>>,
        renderer: Arc<RwLock<Renderer>>,
    ) -> Arc<Server> {
        let server_callback = Arc::new(Mutex::new(None));
        let inner_server = server_callback.clone();
        let mut inner_server = inner_server.lock();
        let _window_size = Arc::new(RwLock::new((0, 0)));
        let render_list =
            Self::spawn_render_list_computer(server_callback.clone(), renderer.clone());
        let server = Arc::new(Server::new(
            protocol::SUPPORTED_PROTOCOLS[0],
            vec![],
            protocol::UUID::default(),
            resources,
            Arc::new(RwLock::new(None)),
            Self::spawn_light_updater(server_callback),
            render_list.0,
            render_list.1,
            Arc::new(RwLock::new(HudContext::new())),
            &renderer.read(),
        ));
        inner_server.replace(server.clone());
        let mut rng = rand::thread_rng();

        for x in (-7 * 16)..(7 * 16) {
            for z in -7 * 16..7 * 16 {
                let h = 5 + (6.0 * (x as f64 / 16.0).cos() * (z as f64 / 16.0).sin()) as i32;
                for y in 0..h {
                    server.world.clone().set_block(
                        Position::new(x, y, z),
                        block::Dirt {
                            snowy: false,
                            variant: block::DirtVariant::Normal,
                        },
                    );
                }
                server
                    .world
                    .clone()
                    .set_block(Position::new(x, h, z), block::Grass { snowy: false });

                if x * x + z * z > 16 * 16 && rng.gen_bool(1.0 / 80.0) {
                    for i in 0..5 {
                        server.world.clone().set_block(
                            Position::new(x, h + 1 + i, z),
                            block::Log {
                                axis: Axis::Y,
                                variant: block::TreeVariant::Oak,
                            },
                        );
                    }
                    for xx in -2..3 {
                        for zz in -2..3 {
                            if xx == 0 && z == 0 {
                                continue;
                            }
                            let world = server.world.clone();
                            world.set_block(
                                Position::new(x + xx, h + 3, z + zz),
                                block::Leaves {
                                    variant: block::TreeVariant::Oak,
                                    check_decay: false,
                                    decayable: false,
                                    distance: 1,
                                },
                            );
                            world.set_block(
                                Position::new(x + xx, h + 4, z + zz),
                                block::Leaves {
                                    variant: block::TreeVariant::Oak,
                                    check_decay: false,
                                    decayable: false,
                                    distance: 1,
                                },
                            );
                            if xx.abs() <= 1 && zz.abs() <= 1 {
                                world.set_block(
                                    Position::new(x + xx, h + 5, z + zz),
                                    block::Leaves {
                                        variant: block::TreeVariant::Oak,
                                        check_decay: false,
                                        decayable: false,
                                        distance: 1,
                                    },
                                );
                            }
                            if xx * xx + zz * zz <= 1 {
                                world.set_block(
                                    Position::new(x + xx, h + 6, z + zz),
                                    block::Leaves {
                                        variant: block::TreeVariant::Oak,
                                        check_decay: false,
                                        decayable: false,
                                        distance: 1,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        server
    }

    fn new(
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        uuid: protocol::UUID,
        resources: Arc<RwLock<resources::Manager>>,
        conn: Arc<RwLock<Option<protocol::Conn>>>,
        light_updater: Sender<LightUpdate>,
        render_list_computer: Sender<bool>,
        render_list_computer_notify: Receiver<bool>,
        hud_context: Arc<RwLock<HudContext>>,
        renderer: &Renderer,
    ) -> Server {
        let mut entities = ecs::Manager::new();
        entity::add_systems(&mut entities);

        let world_entity = entities.get_world();
        let game_info = entities.get_key();
        entities.add_component(world_entity, game_info, entity::GameInfo::new());

        let version = Version::from_id(protocol_version as u32);
        let inventory_context = Arc::new(RwLock::new(InventoryContext::new(
            version,
            renderer,
            hud_context.clone(),
        )));
        hud_context.write().player_inventory =
            Some(inventory_context.read().player_inventory.clone());

        let version = resources.read().version();
        Server {
            uuid,
            conn,
            protocol_version,
            mapped_protocol_version: Version::from_id(protocol_version as u32),
            forge_mods,
            disconnect_data: Arc::new(RwLock::new(DisconnectData::default())),

            world: Arc::new(world::World::new(protocol_version, light_updater)),
            world_data: Arc::new(RwLock::new(WorldData::default())),
            version: RwLock::new(version),
            resources,

            // Entity accessors
            game_info,
            player_movement: entities.get_key(),
            gravity: entities.get_key(),
            position: entities.get_key(),
            target_position: entities.get_key(),
            velocity: entities.get_key(),
            gamemode: entities.get_key(),
            rotation: entities.get_key(),
            target_rotation: entities.get_key(),
            //
            entities: Arc::new(RwLock::new(entities)),
            player: Arc::new(RwLock::new(None)),
            entity_map: Arc::new(RwLock::new(HashMap::with_hasher(
                BuildHasherDefault::default(),
            ))),
            players: Arc::new(RwLock::new(HashMap::with_hasher(
                BuildHasherDefault::default(),
            ))),

            tick_timer: RwLock::from(0.0),
            entity_tick_timer: RwLock::from(0.0),
            received_chat_at: Arc::new(RwLock::new(None)),
            sun_model: RwLock::new(None),

            target_info: Arc::new(RwLock::new(target::Info::new())),
            render_list_computer,
            render_list_computer_notify,
            hud_context,
            inventory_context,
            fps: RwLock::new(0),
            fps_start: RwLock::new(0),
            dead: RwLock::new(false),
            just_died: RwLock::new(false),
            close_death_screen: RwLock::new(false),
            block_break_info: Mutex::new(BlockBreakInfo {
                break_position: Default::default(),
                break_face: BlockDirection::Invalid,
                hardness: 0.0,
                progress: 0.0,
                delay: 0,
                active: false,
            }),
        }
    }

    pub fn disconnect(&self, reason: Option<format::Component>) {
        self.conn.clone().write().take();
        self.disconnect_data.clone().write().disconnect_reason = reason;
        if let Some(player) = self.player.clone().write().take() {
            self.entities.clone().write().remove_entity(player);
        }
        self.disconnect_data.clone().write().just_disconnected = true;
    }

    pub fn is_connected(&self) -> bool {
        let tmp = self.conn.clone();
        return tmp.read().is_some();
    }

    pub fn tick(&self, renderer: Arc<RwLock<render::Renderer>>, delta: f64, game: &mut Game) {
        let start = SystemTime::now();
        let time = start.duration_since(UNIX_EPOCH).unwrap().as_millis();
        if *self.fps_start.read() + 1000 < time {
            self.hud_context
                .clone()
                .write()
                .update_fps(*self.fps.read());
            *self.fps_start.write() = time;
            *self.fps.write() = 0;
        } else {
            *self.fps.write() += 1;
        }
        if *self.close_death_screen.read() {
            *self.close_death_screen.write() = false;
            game.screen_sys.pop_screen();
            game.focused = true;
        }
        let version = self.resources.read().version();
        if version != *self.version.read() {
            *self.version.write() = version;
            self.world.clone().flag_dirty_all();
        }
        let renderer = &mut renderer.write();
        // TODO: Check if the world type actually needs a sun
        if self.sun_model.read().is_none() {
            self.sun_model.write().replace(sun::SunModel::new(renderer));
        }

        // Copy to camera
        if let Some(player) = *self.player.clone().read() {
            let position = self
                .entities
                .clone()
                .read()
                .get_component(player, self.position)
                .unwrap();
            let rotation = self
                .entities
                .clone()
                .read()
                .get_component(player, self.rotation)
                .unwrap();
            renderer.camera.pos =
                cgmath::Point3::from_vec(position.position + cgmath::Vector3::new(0.0, 1.62, 0.0));
            renderer.camera.yaw = rotation.yaw;
            renderer.camera.pitch = rotation.pitch;
        }
        self.entity_tick(renderer, delta, game.focused, *self.dead.read());

        *self.tick_timer.write() += delta;
        while *self.tick_timer.read() >= 3.0 && self.is_connected() {
            self.minecraft_tick();
            *self.tick_timer.write() -= 3.0;
        }

        self.update_time(renderer, delta);
        if let Some(sun_model) = self.sun_model.write().as_mut() {
            sun_model.tick(
                renderer,
                self.world_data.clone().read().world_time,
                self.world_data.clone().read().world_age,
            );
        }
        let world = self.world.clone();
        world.tick(&mut self.entities.clone().write());

        if self.player.clone().read().is_some() {
            if *self.just_died.read() {
                *self.just_died.write() = false;
                while game.screen_sys.is_current_closable() {
                    game.screen_sys.pop_screen();
                }
                game.screen_sys.add_screen(Box::new(Respawn::new(0))); // TODO: Use the correct score!
                game.focused = false;
            }
            let world = self.world.clone();
            if let Some((pos, bl, _, _)) = target::trace_ray(
                &world,
                4.0,
                renderer.camera.pos.to_vec(),
                renderer.view_vector.cast().unwrap(),
                target::test_block,
            ) {
                self.target_info.clone().write().update(renderer, pos, bl);
            } else {
                self.target_info.clone().write().clear(renderer);
            }
        } else {
            self.target_info.clone().write().clear(renderer);
        }
    }

    fn entity_tick(&self, renderer: &mut render::Renderer, delta: f64, focused: bool, dead: bool) {
        let world_entity = self.entities.clone().read().get_world();
        // Update the game's state for entities to read
        self.entities
            .clone()
            .write()
            .get_component_mut(world_entity, self.game_info)
            .unwrap()
            .delta = delta;

        if self.is_connected() || self.disconnect_data.clone().read().just_disconnected {
            // Allow an extra tick when disconnected to clean up
            self.disconnect_data.clone().write().just_disconnected = false;
            *self.entity_tick_timer.write() += delta;
            while *self.entity_tick_timer.read() >= 3.0 {
                let world = self.world.clone();
                self.entities
                    .clone()
                    .write()
                    .tick(&world, renderer, focused, dead);
                *self.entity_tick_timer.write() -= 3.0;
            }
            let world = self.world.clone();
            self.entities
                .clone()
                .write()
                .render_tick(&world, renderer, focused, dead);
        }
    }

    pub fn remove(&mut self, renderer: &mut render::Renderer) {
        let world = self.world.clone();
        self.entities
            .clone()
            .write()
            .remove_all_entities(&world, renderer);
        if let Some(sun_model) = self.sun_model.write().as_mut() {
            sun_model.remove(renderer);
        }
        self.target_info.clone().write().clear(renderer);
    }

    fn update_time(&self, renderer: &mut render::Renderer, delta: f64) {
        if self.world_data.clone().read().tick_time {
            self.world_data.clone().write().world_time_target += delta / 3.0;
            let time = self.world_data.clone().read().world_time_target;
            self.world_data.clone().write().world_time_target = (24000.0 + time) % 24000.0;
            let mut diff = self.world_data.clone().read().world_time_target
                - self.world_data.clone().read().world_time;
            if diff < -12000.0 {
                diff += 24000.0
            } else if diff > 12000.0 {
                diff -= 24000.0
            }
            self.world_data.clone().write().world_time += diff * (1.5 / 60.0) * delta;
            let time = self.world_data.clone().read().world_time;
            self.world_data.clone().write().world_time = (24000.0 + time) % 24000.0;
        } else {
            let time = self.world_data.clone().read().world_time_target;
            self.world_data.clone().write().world_time = time;
        }
        renderer.sky_offset = self.calculate_sky_offset();
    }

    fn calculate_sky_offset(&self) -> f32 {
        use std::f32::consts::PI;
        let mut offset =
            ((1.0 + self.world_data.clone().read().world_time as f32) / 24000.0) - 0.25;
        if offset < 0.0 {
            offset += 1.0;
        } else if offset > 1.0 {
            offset -= 1.0;
        }

        let prev_offset = offset;
        offset = 1.0 - (((offset * PI).cos() + 1.0) / 2.0);
        offset = prev_offset + (offset - prev_offset) / 3.0;

        offset = 1.0 - ((offset * PI * 2.0).cos() * 2.0 + 0.2);
        if offset > 1.0 {
            offset = 1.0;
        } else if offset < 0.0 {
            offset = 0.0;
        }
        offset = 1.0 - offset;
        offset * 0.8 + 0.2
    }

    pub fn minecraft_tick(&self) {
        use std::f32::consts::PI;
        if let Some(player) = *self.player.clone().write() {
            let movement = self
                .entities
                .clone()
                .write()
                .get_component_mut(player, self.player_movement)
                .unwrap();
            let on_ground = self
                .entities
                .clone()
                .read()
                .get_component(player, self.gravity)
                .map_or(false, |v| v.on_ground);
            let position = self
                .entities
                .clone()
                .read()
                .get_component(player, self.target_position)
                .unwrap();
            let rotation = self
                .entities
                .clone()
                .read()
                .get_component(player, self.rotation)
                .unwrap();

            // Force the server to know when touched the ground
            // otherwise if it happens between ticks the server
            // will think we are flying.
            let on_ground = if movement.did_touch_ground {
                movement.did_touch_ground = false;
                true
            } else {
                on_ground
            };

            // Sync our position to the server
            // Use the smaller packets when possible
            if self.protocol_version >= 47 {
                let packet = packet::play::serverbound::PlayerPositionLook {
                    x: position.position.x,
                    y: position.position.y,
                    z: position.position.z,
                    yaw: -(rotation.yaw as f32) * (180.0 / PI),
                    pitch: (-rotation.pitch as f32) * (180.0 / PI) + 180.0,
                    on_ground,
                };
                self.write_packet(packet);
            } else {
                let packet = packet::play::serverbound::PlayerPositionLook_HeadY {
                    x: position.position.x,
                    feet_y: position.position.y,
                    head_y: position.position.y + 1.62,
                    z: position.position.z,
                    yaw: -(rotation.yaw as f32) * (180.0 / PI),
                    pitch: (-rotation.pitch as f32) * (180.0 / PI) + 180.0,
                    on_ground,
                };
                self.write_packet(packet);
            }
        }
        let break_delay = self.block_break_info.lock().delay;
        if break_delay > 0 {
            self.block_break_info.lock().delay -= 1;
        } else if self.block_break_info.lock().active {
            if self.block_break_info.lock().progress >= 1.0 {
                let face_idx = self.block_break_info.lock().break_face.index() as u8;
                if self.mapped_protocol_version < Version::V1_8 {
                    let pos = self.block_break_info.lock().break_position;
                    self.write_packet(packet::play::serverbound::PlayerDigging_u8_u8y {
                        status: DigType::StopDestroyBlock.ordinal(),
                        x: pos.x,
                        y: pos.y as u8,
                        z: pos.z,
                        face: face_idx,
                    });
                } else if self.mapped_protocol_version < Version::V1_9 {
                    self.write_packet(packet::play::serverbound::PlayerDigging_u8 {
                        status: DigType::StopDestroyBlock.ordinal(),
                        location: self.block_break_info.lock().break_position,
                        face: face_idx,
                    });
                } else {
                    self.write_packet(packet::play::serverbound::PlayerDigging {
                        status: VarInt(DigType::StopDestroyBlock.ordinal() as i32),
                        location: self.block_break_info.lock().break_position,
                        face: face_idx,
                    });
                }
                self.block_break_info.lock().active = false;
                self.block_break_info.lock().delay = 5;
            } else {
                if self.mapped_protocol_version < Version::V1_8 {
                    self.write_packet(packet::play::serverbound::ArmSwing_Handsfree_ID {
                        entity_id: 0, // TODO: Check these values!
                        animation: 0,
                    });
                } else if self.mapped_protocol_version < Version::V1_9 {
                    self.write_packet(packet::play::serverbound::ArmSwing_Handsfree { empty: () })
                } else {
                    self.write_packet(packet::play::serverbound::ArmSwing {
                        hand: protocol::VarInt(0),
                    });
                }
                self.block_break_info.lock().progress += 0.1; // TODO: Make this value meaningful
            }
        }
    }

    pub fn key_press(
        &self,
        down: bool,
        key: Actionkey,
        screen_sys: &mut ScreenSystem,
        focused: &mut bool,
    ) {
        if *focused || key == Actionkey::OpenInv {
            let mut state_changed = false;
            if let Some(player) = *self.player.clone().write() {
                if let Some(movement) = self
                    .entities
                    .clone()
                    .write()
                    .get_component_mut(player, self.player_movement)
                {
                    state_changed = movement.pressed_keys.get(&key).map_or(false, |v| *v) != down;
                    movement.pressed_keys.insert(key, down);
                }
            }
            match key {
                Actionkey::OpenInv => {
                    if down && state_changed {
                        if self.inventory_context.clone().read().inventory.is_some() {
                            screen_sys.pop_screen();
                            *focused = true;
                        } else if *focused {
                            let player_inv = self
                                .inventory_context
                                .clone()
                                .read()
                                .player_inventory
                                .clone();
                            screen_sys.add_screen(Box::new(
                                render::inventory::InventoryWindow::new(
                                    player_inv,
                                    self.inventory_context.clone(),
                                ),
                            ));
                            *focused = false;
                        }
                    }
                }
                Actionkey::ToggleHud => {
                    if down && state_changed {
                        let curr = self.hud_context.read().enabled;
                        self.hud_context.write().enabled = !curr;
                    }
                }
                Actionkey::ToggleDebug => {
                    if down && state_changed {
                        let curr = self.hud_context.read().debug;
                        self.hud_context.write().debug = !curr;
                    }
                }
                _ => {}
            };
        }
    }

    pub fn on_left_click(&self, renderer: Arc<RwLock<render::Renderer>>) {
        if self.mapped_protocol_version < Version::V1_8 {
            self.write_packet(packet::play::serverbound::ArmSwing_Handsfree_ID {
                entity_id: 0, // TODO: Check these values!
                animation: 0,
            });
        } else if self.mapped_protocol_version < Version::V1_9 {
            self.write_packet(packet::play::serverbound::ArmSwing_Handsfree { empty: () })
        } else {
            self.write_packet(packet::play::serverbound::ArmSwing {
                hand: protocol::VarInt(0),
            });
        }
        // TODO: Implement clientside animation.
        if self.player.clone().read().is_some() {
            let world = self.world.clone();
            let gamemode = *self
                .entities
                .clone()
                .write()
                .get_component(*self.player.clone().read().as_ref().unwrap(), self.gamemode)
                .unwrap();
            if !matches!(gamemode, GameMode::Spectator) && self.block_break_info.lock().delay == 0 {
                // TODO: Check this
                if let Some((pos, _, face, _)) = target::trace_ray(
                    &world,
                    4.0,
                    renderer.read().camera.pos.to_vec(),
                    renderer.read().view_vector.cast().unwrap(),
                    target::test_block,
                ) {
                    if self.mapped_protocol_version < Version::V1_8 {
                        self.write_packet(packet::play::serverbound::PlayerDigging_u8_u8y {
                            status: DigType::StartDestroyBlock.ordinal(),
                            x: pos.x,
                            y: pos.y as u8,
                            z: pos.z,
                            face: face.index() as u8,
                        });
                    } else if self.mapped_protocol_version < Version::V1_9 {
                        self.write_packet(packet::play::serverbound::PlayerDigging_u8 {
                            status: DigType::StartDestroyBlock.ordinal(),
                            location: pos,
                            face: face.index() as u8,
                        });
                    } else {
                        self.write_packet(packet::play::serverbound::PlayerDigging {
                            status: VarInt(DigType::StartDestroyBlock.ordinal() as i32),
                            location: pos,
                            face: face.index() as u8,
                        });
                    }
                    self.block_break_info.lock().break_face = face;
                    self.block_break_info.lock().break_position = pos;
                    self.block_break_info.lock().progress = 0.0;
                    self.block_break_info.lock().hardness = 1.0; // TODO: Get actual hardness values depending on blocktype and version and tool in hands
                    self.block_break_info.lock().active = true;
                }
            }
        }
    }

    pub fn on_release_left_click(&self) {
        self.abort_breaking();
    }

    pub fn abort_breaking(&self) {
        // TODO: Call this on hand switching (hotbar scrolling)
        if self.block_break_info.lock().active {
            self.block_break_info.lock().active = false;
            self.block_break_info.lock().delay = 5;
            let pos = self.block_break_info.lock().break_position;
            let face = self.block_break_info.lock().break_face;
            if self.mapped_protocol_version < Version::V1_8 {
                self.write_packet(packet::play::serverbound::PlayerDigging_u8_u8y {
                    status: DigType::AbortDestroyBlock.ordinal(),
                    x: pos.x,
                    y: pos.y as u8,
                    z: pos.z,
                    face: face.index() as u8,
                });
            } else if self.mapped_protocol_version < Version::V1_9 {
                self.write_packet(packet::play::serverbound::PlayerDigging_u8 {
                    status: DigType::AbortDestroyBlock.ordinal(),
                    location: pos,
                    face: face.index() as u8,
                });
            } else {
                self.write_packet(packet::play::serverbound::PlayerDigging {
                    status: VarInt(DigType::AbortDestroyBlock.ordinal() as i32),
                    location: pos,
                    face: face.index() as u8,
                });
            }
        }
    }

    pub fn on_right_click(&self, renderer: Arc<RwLock<render::Renderer>>) {
        if self.player.clone().read().is_some() {
            let world = self.world.clone();
            let gamemode = *self
                .entities
                .clone()
                .write()
                .get_component(*self.player.clone().read().as_ref().unwrap(), self.gamemode)
                .unwrap();
            if !matches!(gamemode, GameMode::Spectator) {
                // TODO: Check this
                if let Some((pos, _, face, at)) = target::trace_ray(
                    &world,
                    4.0,
                    renderer.read().camera.pos.to_vec(),
                    renderer.read().view_vector.cast().unwrap(),
                    target::test_block,
                ) {
                    if self.protocol_version >= 477 {
                        self.write_packet(
                            packet::play::serverbound::PlayerBlockPlacement_insideblock {
                                location: pos,
                                face: protocol::VarInt(face.index() as i32),
                                hand: protocol::VarInt(0),
                                cursor_x: at.x as f32,
                                cursor_y: at.y as f32,
                                cursor_z: at.z as f32,
                                inside_block: false,
                            },
                        );
                    } else if self.protocol_version >= 315 {
                        self.write_packet(packet::play::serverbound::PlayerBlockPlacement_f32 {
                            location: pos,
                            face: protocol::VarInt(face.index() as i32),
                            hand: protocol::VarInt(0),
                            cursor_x: at.x as f32,
                            cursor_y: at.y as f32,
                            cursor_z: at.z as f32,
                        });
                    } else if self.protocol_version >= 49 {
                        self.write_packet(packet::play::serverbound::PlayerBlockPlacement_u8 {
                            location: pos,
                            face: protocol::VarInt(face.index() as i32),
                            hand: protocol::VarInt(0),
                            cursor_x: (at.x * 16.0) as u8,
                            cursor_y: (at.y * 16.0) as u8,
                            cursor_z: (at.z * 16.0) as u8,
                        });
                    } else if self.protocol_version >= 47 {
                        let item = self
                            .hud_context
                            .clone()
                            .read()
                            .player_inventory
                            .as_ref()
                            .unwrap()
                            .clone()
                            .read()
                            .get_item(
                                (36 + self.hud_context.clone().read().get_slot_index()) as i16,
                            )
                            .as_ref()
                            .map(|item| item.stack.clone());
                        self.write_packet(
                            packet::play::serverbound::PlayerBlockPlacement_u8_Item {
                                location: pos,
                                face: face.index() as u8,
                                hand: item,
                                cursor_x: (at.x * 16.0) as u8,
                                cursor_y: (at.y * 16.0) as u8,
                                cursor_z: (at.z * 16.0) as u8,
                            },
                        );
                    } else {
                        let item = self
                            .hud_context
                            .clone()
                            .read()
                            .player_inventory
                            .as_ref()
                            .unwrap()
                            .clone()
                            .read()
                            .get_item(
                                (36 + self.hud_context.clone().read().get_slot_index()) as i16,
                            )
                            .as_ref()
                            .map(|item| item.stack.clone());
                        self.write_packet(
                            packet::play::serverbound::PlayerBlockPlacement_u8_Item_u8y {
                                x: pos.x,
                                y: pos.y as u8,
                                z: pos.x,
                                face: face.index() as u8,
                                hand: item,
                                cursor_x: (at.x * 16.0) as u8,
                                cursor_y: (at.y * 16.0) as u8,
                                cursor_z: (at.z * 16.0) as u8,
                            },
                        );
                    }
                    if self.mapped_protocol_version < Version::V1_8 {
                        self.write_packet(packet::play::serverbound::ArmSwing_Handsfree_ID {
                            entity_id: 0,
                            animation: 0,
                        });
                    } else if self.mapped_protocol_version < Version::V1_9 {
                        self.write_packet(packet::play::serverbound::ArmSwing_Handsfree {
                            empty: (),
                        })
                    } else {
                        self.write_packet(packet::play::serverbound::ArmSwing {
                            hand: protocol::VarInt(0),
                        });
                    }
                }
            }
        }
    }

    pub fn write_packet<T: protocol::PacketType>(&self, p: T) {
        let conn = self.conn.clone();
        let mut conn = conn.write();
        if conn.is_some() {
            let result = conn.as_mut().unwrap().write_packet(p);
            if result.is_ok() {
                return;
            }
        }
        self.disconnect(Some(Component::Text(TextComponent {
            // TODO: Test this!
            text: "Already disconnected!".to_string(),
            modifier: Default::default(),
        })));
    }

    fn on_keep_alive_i64(&self, keep_alive: packet::play::clientbound::KeepAliveClientbound_i64) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_i64 {
            id: keep_alive.id,
        });
    }

    fn on_keep_alive_varint(
        &self,
        keep_alive: packet::play::clientbound::KeepAliveClientbound_VarInt,
    ) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_VarInt {
            id: keep_alive.id,
        });
    }

    fn on_keep_alive_i32(&self, keep_alive: packet::play::clientbound::KeepAliveClientbound_i32) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_i32 {
            id: keep_alive.id,
        });
    }

    fn on_plugin_message_clientbound_i16(
        &self,
        msg: packet::play::clientbound::PluginMessageClientbound_i16,
    ) {
        self.on_plugin_message_clientbound(&msg.channel, msg.data.data.as_slice())
    }

    fn on_plugin_message_clientbound_1(
        &self,
        msg: packet::play::clientbound::PluginMessageClientbound,
    ) {
        self.on_plugin_message_clientbound(&msg.channel, &msg.data)
    }

    fn on_plugin_message_clientbound(&self, channel: &str, data: &[u8]) {
        if protocol::is_network_debug() {
            debug!(
                "Received plugin message: channel={}, data={:?}",
                channel, data
            );
        }

        match channel {
            "REGISTER" => {}   // TODO
            "UNREGISTER" => {} // TODO
            "FML|HS" => {
                let msg = crate::protocol::Serializable::read_from(&mut std::io::Cursor::new(data))
                    .unwrap();
                // debug!("FML|HS msg={:?}", msg);

                use forge::FmlHs::*;
                use forge::Phase::*;
                match msg {
                    ServerHello {
                        fml_protocol_version,
                        override_dimension,
                    } => {
                        debug!(
                            "Received FML|HS ServerHello {} {:?}",
                            fml_protocol_version, override_dimension
                        );

                        self.write_plugin_message("REGISTER", b"FML|HS\0FML\0FML|MP\0FML\0FORGE");
                        self.write_fmlhs_plugin_message(&ClientHello {
                            fml_protocol_version,
                        });
                        // Send stashed mods list received from ping packet, client matching server
                        let mods = crate::protocol::LenPrefixed::<
                            crate::protocol::VarInt,
                            forge::ForgeMod,
                        >::new(self.forge_mods.clone());
                        self.write_fmlhs_plugin_message(&ModList { mods });
                    }
                    ModList { mods } => {
                        debug!("Received FML|HS ModList: {:?}", mods);

                        self.write_fmlhs_plugin_message(&HandshakeAck {
                            phase: WaitingServerData,
                        });
                    }
                    ModIdData {
                        mappings,
                        block_substitutions: _,
                        item_substitutions: _,
                    } => {
                        debug!("Received FML|HS ModIdData");
                        for m in mappings.data {
                            let (namespace, name) = m.name.split_at(1);
                            if namespace == protocol::forge::BLOCK_NAMESPACE {
                                self.world
                                    .clone()
                                    .modded_block_ids
                                    .clone()
                                    .write()
                                    .insert(m.id.0 as usize, name.to_string());
                            }
                        }
                        self.write_fmlhs_plugin_message(&HandshakeAck {
                            phase: WaitingServerComplete,
                        });
                    }
                    RegistryData {
                        has_more,
                        name,
                        ids,
                        substitutions: _,
                        dummies: _,
                    } => {
                        debug!("Received FML|HS RegistryData for {}", name);
                        if name == "minecraft:blocks" {
                            for m in ids.data {
                                self.world
                                    .clone()
                                    .modded_block_ids
                                    .clone()
                                    .write()
                                    .insert(m.id.0 as usize, m.name);
                            }
                        }
                        if !has_more {
                            self.write_fmlhs_plugin_message(&HandshakeAck {
                                phase: WaitingServerComplete,
                            });
                        }
                    }
                    HandshakeAck { phase } => match phase {
                        WaitingCAck => {
                            self.write_fmlhs_plugin_message(&HandshakeAck {
                                phase: PendingComplete,
                            });
                        }
                        Complete => {
                            debug!("FML|HS handshake complete!");
                        }
                        _ => unimplemented!(),
                    },
                    _ => (),
                }
            }
            _ => (),
        }
    }

    fn write_fmlhs_plugin_message(&self, msg: &forge::FmlHs) {
        let _ = self
            .conn
            .clone()
            .write()
            .as_mut()
            .unwrap()
            .write_fmlhs_plugin_message(msg); // TODO handle errors
    }

    fn write_plugin_message(&self, channel: &str, data: &[u8]) {
        let _ = self
            .conn
            .clone()
            .write()
            .as_mut()
            .unwrap()
            .write_plugin_message(channel, data); // TODO handle errors
    }

    fn on_game_join_worldnames_ishard(
        &self,
        join: packet::play::clientbound::JoinGame_WorldNames_IsHard,
    ) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_worldnames(&self, join: packet::play::clientbound::JoinGame_WorldNames) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_hashedseed_respawn(
        &self,
        join: packet::play::clientbound::JoinGame_HashedSeed_Respawn,
    ) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_i32_viewdistance(
        &self,
        join: packet::play::clientbound::JoinGame_i32_ViewDistance,
    ) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_i32(&self, join: packet::play::clientbound::JoinGame_i32) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_i8(&self, join: packet::play::clientbound::JoinGame_i8) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join_i8_nodebug(&self, join: packet::play::clientbound::JoinGame_i8_NoDebug) {
        self.on_game_join(join.gamemode, join.entity_id)
    }

    fn on_game_join(&self, gamemode: u8, entity_id: i32) {
        let gamemode = GameMode::from_int((gamemode & 0x7) as i32);
        let player = entity::player::create_local(&mut self.entities.clone().write());
        if let Some(info) = self.players.clone().read().get(&self.uuid) {
            let model = self
                .entities
                .clone()
                .write()
                .get_component_mut_direct::<entity::player::PlayerModel>(player)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.hud_context.clone().write().update_game_mode(gamemode);
        *self
            .entities
            .clone()
            .write()
            .get_component_mut(player, self.gamemode)
            .unwrap() = gamemode;
        self.entities
            .clone()
            .write()
            .get_component_mut(player, self.player_movement)
            .unwrap()
            .flying = gamemode.can_fly();

        self.entity_map.clone().write().insert(entity_id, player);
        self.player.clone().write().replace(player);

        // Let the server know who we are
        let brand = plugin_messages::Brand {
            brand: "leafish".into(),
        };
        // TODO: refactor with write_plugin_message
        if self.protocol_version >= 47 {
            self.write_packet(brand.into_message());
        } else {
            self.write_packet(brand.into_message17());
        }
        if self.protocol_version <= 48 {
            // 1 snapshot after 1.8
            self.write_packet(ClientSettings_u8_Handsfree {
                locale: "en_us".to_string(), // TODO: Make this configurable!
                view_distance: 8,            // TODO: Make this configurable!
                chat_mode: 0,                // TODO: Make this configurable!
                chat_colors: true,           // TODO: Make this configurable!
                displayed_skin_parts: 127,   // TODO: Make this configurable!
            });
        } else {
            self.write_packet(ClientSettings {
                locale: "en_us".to_string(),   // TODO: Make this configurable!
                view_distance: 8,              // TODO: Make this configurable!
                chat_mode: Default::default(), // TODO: Make this configurable!
                chat_colors: true,             // TODO: Make this configurable!
                displayed_skin_parts: 127,     // TODO: Make this configurable!
                main_hand: Default::default(), // TODO: Make this configurable!
            });
        }
    }

    fn on_respawn_hashedseed(&self, respawn: packet::play::clientbound::Respawn_HashedSeed) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_gamemode(&self, respawn: packet::play::clientbound::Respawn_Gamemode) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_worldname(&self, respawn: packet::play::clientbound::Respawn_WorldName) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_nbt(&self, respawn: packet::play::clientbound::Respawn_NBT) {
        self.respawn(respawn.gamemode)
    }

    fn respawn(&self, gamemode_u8: u8) {
        let gamemode = GameMode::from_int((gamemode_u8 & 0x7) as i32);

        if let Some(player) = *self.player.clone().write() {
            self.hud_context.clone().write().update_game_mode(gamemode);
            *self
                .entities
                .clone()
                .write()
                .get_component_mut(player, self.gamemode)
                .unwrap() = gamemode;
            self.entities
                .clone()
                .write()
                .get_component_mut(player, self.player_movement)
                .unwrap()
                .flying = gamemode.can_fly();
        }
        if *self.dead.read() {
            *self.close_death_screen.write() = true;
            *self.dead.write() = false;
            *self.just_died.write() = false;
            self.hud_context
                .clone()
                .write()
                .update_health_and_food(20.0, 20, 0); // TODO: Verify this!
            self.hud_context.clone().write().update_slot_index(0);
            self.hud_context.clone().write().update_exp(0.0, 0);
            self.hud_context.clone().write().update_absorbtion(0.0);
            self.hud_context.clone().write().update_armor(0);
            // self.hud_context.clone().write().update_breath(-1); // TODO: Fix this!
        }
    }

    // TODO: make use of "on_disconnect"
    #[allow(dead_code)]
    fn on_disconnect(&self, disconnect: packet::play::clientbound::Disconnect) {
        self.disconnect(Some(disconnect.reason));
    }

    fn on_time_update(&self, time_update: packet::play::clientbound::TimeUpdate) {
        self.world_data.clone().write().world_age = time_update.time_of_day;
        self.world_data.clone().write().world_time_target =
            (time_update.time_of_day % 24000) as f64;
        if self.world_data.clone().read().world_time_target < 0.0 {
            self.world_data.clone().write().world_time_target *= -1.0;
            self.world_data.clone().write().tick_time = false;
        } else {
            self.world_data.clone().write().tick_time = true;
        }
    }

    fn on_game_state_change(&self, game_state: packet::play::clientbound::ChangeGameState) {
        if game_state.reason == 3 {
            if let Some(player) = *self.player.write() {
                let gamemode = GameMode::from_int(game_state.value as i32);
                self.hud_context.clone().write().update_game_mode(gamemode);
                *self
                    .entities
                    .clone()
                    .write()
                    .get_component_mut(player, self.gamemode)
                    .unwrap() = gamemode;
                self.entities
                    .clone()
                    .write()
                    .get_component_mut(player, self.player_movement)
                    .unwrap()
                    .flying = gamemode.can_fly();
            }
        }
    }

    fn on_entity_destroy(&self, entity_destroy: packet::play::clientbound::EntityDestroy) {
        for id in entity_destroy.entity_ids.data {
            if let Some(entity) = self.entity_map.clone().write().remove(&id.0) {
                self.entities.clone().write().remove_entity(entity);
            }
        }
    }

    fn on_entity_destroy_u8(&self, entity_destroy: packet::play::clientbound::EntityDestroy_u8) {
        for id in entity_destroy.entity_ids.data {
            if let Some(entity) = self.entity_map.clone().write().remove(&id) {
                self.entities.clone().write().remove_entity(entity);
            }
        }
    }

    fn on_entity_teleport_f64(
        &self,
        entity_telport: packet::play::clientbound::EntityTeleport_f64,
    ) {
        self.on_entity_teleport(
            entity_telport.entity_id.0,
            entity_telport.x,
            entity_telport.y,
            entity_telport.z,
            entity_telport.yaw as f64,
            entity_telport.pitch as f64,
            entity_telport.on_ground,
        )
    }

    fn on_entity_teleport_i32(
        &self,
        entity_telport: packet::play::clientbound::EntityTeleport_i32,
    ) {
        self.on_entity_teleport(
            entity_telport.entity_id.0,
            f64::from(entity_telport.x),
            f64::from(entity_telport.y),
            f64::from(entity_telport.z),
            entity_telport.yaw as f64,
            entity_telport.pitch as f64,
            entity_telport.on_ground,
        )
    }

    fn on_entity_teleport_i32_i32_noground(
        &self,
        entity_telport: packet::play::clientbound::EntityTeleport_i32_i32_NoGround,
    ) {
        let on_ground = true; // TODO: how is this supposed to be set? (for 1.7)
        self.on_entity_teleport(
            entity_telport.entity_id,
            f64::from(entity_telport.x),
            f64::from(entity_telport.y),
            f64::from(entity_telport.z),
            entity_telport.yaw as f64,
            entity_telport.pitch as f64,
            on_ground,
        )
    }

    fn on_entity_teleport(
        &self,
        entity_id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        _on_ground: bool,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().get(&entity_id) {
            let target_position = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            let target_rotation = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
            target_rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            target_rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_move_i16(&self, m: packet::play::clientbound::EntityMove_i16) {
        self.on_entity_move(
            m.entity_id.0,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }

    fn on_entity_move_i8(&self, m: packet::play::clientbound::EntityMove_i8) {
        self.on_entity_move(
            m.entity_id.0,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }

    fn on_entity_move_i8_i32_noground(
        &self,
        m: packet::play::clientbound::EntityMove_i8_i32_NoGround,
    ) {
        self.on_entity_move(
            m.entity_id,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }

    fn on_entity_move(&self, entity_id: i32, delta_x: f64, delta_y: f64, delta_z: f64) {
        if let Some(entity) = self.entity_map.clone().read().get(&entity_id) {
            let position = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
        }
    }

    fn on_entity_look(&self, entity_id: i32, yaw: f64, pitch: f64) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().get(&entity_id) {
            let rotation = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_look_varint(&self, look: packet::play::clientbound::EntityLook_VarInt) {
        self.on_entity_look(look.entity_id.0, look.yaw as f64, look.pitch as f64)
    }

    fn on_entity_look_i32_noground(
        &self,
        look: packet::play::clientbound::EntityLook_i32_NoGround,
    ) {
        self.on_entity_look(look.entity_id, look.yaw as f64, look.pitch as f64)
    }

    fn on_entity_look_and_move_i16(
        &self,
        lookmove: packet::play::clientbound::EntityLookAndMove_i16,
    ) {
        self.on_entity_look_and_move(
            lookmove.entity_id.0,
            f64::from(lookmove.delta_x),
            f64::from(lookmove.delta_y),
            f64::from(lookmove.delta_z),
            lookmove.yaw as f64,
            lookmove.pitch as f64,
        )
    }

    fn on_entity_look_and_move_i8(
        &self,
        lookmove: packet::play::clientbound::EntityLookAndMove_i8,
    ) {
        self.on_entity_look_and_move(
            lookmove.entity_id.0,
            f64::from(lookmove.delta_x),
            f64::from(lookmove.delta_y),
            f64::from(lookmove.delta_z),
            lookmove.yaw as f64,
            lookmove.pitch as f64,
        )
    }

    fn on_entity_look_and_move_i8_i32_noground(
        &self,
        lookmove: packet::play::clientbound::EntityLookAndMove_i8_i32_NoGround,
    ) {
        self.on_entity_look_and_move(
            lookmove.entity_id,
            f64::from(lookmove.delta_x),
            f64::from(lookmove.delta_y),
            f64::from(lookmove.delta_z),
            lookmove.yaw as f64,
            lookmove.pitch as f64,
        )
    }

    fn on_entity_look_and_move(
        &self,
        entity_id: i32,
        delta_x: f64,
        delta_y: f64,
        delta_z: f64,
        yaw: f64,
        pitch: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().get(&entity_id) {
            let position = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            let rotation = self
                .entities
                .clone()
                .write()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_player_spawn_f64_nometa(&self, spawn: packet::play::clientbound::SpawnPlayer_f64_NoMeta) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            spawn.x,
            spawn.y,
            spawn.z,
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }

    fn on_player_spawn_f64(&self, spawn: packet::play::clientbound::SpawnPlayer_f64) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            spawn.x,
            spawn.y,
            spawn.z,
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }

    fn on_player_spawn_i32(&self, spawn: packet::play::clientbound::SpawnPlayer_i32) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            f64::from(spawn.x),
            f64::from(spawn.y),
            f64::from(spawn.z),
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }

    fn on_player_spawn_i32_helditem(
        &self,
        spawn: packet::play::clientbound::SpawnPlayer_i32_HeldItem,
    ) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            f64::from(spawn.x),
            f64::from(spawn.y),
            f64::from(spawn.z),
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }

    fn on_player_spawn_i32_helditem_string(
        &self,
        spawn: packet::play::clientbound::SpawnPlayer_i32_HeldItem_String,
    ) {
        // 1.7.10: populate the player list here, since we only now know the UUID
        let uuid = protocol::UUID::from_str(&spawn.uuid).unwrap();
        self.players
            .clone()
            .write()
            .entry(uuid.clone())
            .or_insert(PlayerInfo {
                name: spawn.name.clone(),
                uuid,
                skin_url: None,

                display_name: None,
                ping: 0, // TODO: don't overwrite from PlayerInfo_String
                gamemode: GameMode::from_int(0),
            });

        self.on_player_spawn(
            spawn.entity_id.0,
            protocol::UUID::from_str(&spawn.uuid).unwrap(),
            f64::from(spawn.x),
            f64::from(spawn.y),
            f64::from(spawn.z),
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }

    fn on_player_spawn(
        &self,
        entity_id: i32,
        uuid: protocol::UUID,
        x: f64,
        y: f64,
        z: f64,
        pitch: f64,
        yaw: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().write().remove(&entity_id) {
            self.entities.clone().write().remove_entity(entity);
        }
        let entity = entity::player::create_remote(
            &mut self.entities.clone().write(),
            self.players
                .clone()
                .read()
                .get(&uuid)
                .map_or("MISSING", |v| &v.name),
        );
        let position = self
            .entities
            .clone()
            .write()
            .get_component_mut(entity, self.position)
            .unwrap();
        let target_position = self
            .entities
            .clone()
            .write()
            .get_component_mut(entity, self.target_position)
            .unwrap();
        let rotation = self
            .entities
            .clone()
            .write()
            .get_component_mut(entity, self.rotation)
            .unwrap();
        let target_rotation = self
            .entities
            .clone()
            .write()
            .get_component_mut(entity, self.target_rotation)
            .unwrap();
        position.position.x = x;
        position.position.y = y;
        position.position.z = z;
        target_position.position.x = x;
        target_position.position.y = y;
        target_position.position.z = z;
        rotation.yaw = -(yaw / 256.0) * PI * 2.0;
        rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        target_rotation.yaw = rotation.yaw;
        target_rotation.pitch = rotation.pitch;
        if let Some(info) = self.players.clone().read().get(&uuid) {
            let model = self
                .entities
                .clone()
                .write()
                .get_component_mut_direct::<entity::player::PlayerModel>(entity)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.entity_map.clone().write().insert(entity_id, entity);
    }

    fn on_teleport_player_withconfirm(
        &self,
        teleport: packet::play::clientbound::TeleportPlayer_WithConfirm,
    ) {
        self.on_teleport_player(
            teleport.x,
            teleport.y,
            teleport.z,
            teleport.yaw as f64,
            teleport.pitch as f64,
            teleport.flags,
            Some(teleport.teleport_id),
        )
    }

    fn on_teleport_player_noconfirm(
        &self,
        teleport: packet::play::clientbound::TeleportPlayer_NoConfirm,
    ) {
        self.on_teleport_player(
            teleport.x,
            teleport.y,
            teleport.z,
            teleport.yaw as f64,
            teleport.pitch as f64,
            teleport.flags,
            None,
        )
    }

    fn on_teleport_player_onground(
        &self,
        teleport: packet::play::clientbound::TeleportPlayer_OnGround,
    ) {
        let flags: u8 = 0; // always absolute
        self.on_teleport_player(
            teleport.x,
            teleport.eyes_y - 1.62,
            teleport.z,
            teleport.yaw as f64,
            teleport.pitch as f64,
            flags,
            None,
        )
    }

    fn on_teleport_player(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        flags: u8,
        teleport_id: Option<protocol::VarInt>,
    ) {
        use std::f64::consts::PI;
        if let Some(player) = *self.player.clone().write() {
            let position = self
                .entities
                .clone()
                .write()
                .get_component_mut(player, self.target_position)
                .unwrap();
            let rotation = self
                .entities
                .clone()
                .write()
                .get_component_mut(player, self.rotation)
                .unwrap();
            let velocity = self
                .entities
                .clone()
                .write()
                .get_component_mut(player, self.velocity)
                .unwrap();

            position.position.x =
                calculate_relative_teleport(TeleportFlag::RelX, flags, position.position.x, x);
            position.position.y =
                calculate_relative_teleport(TeleportFlag::RelY, flags, position.position.y, y);
            position.position.z =
                calculate_relative_teleport(TeleportFlag::RelZ, flags, position.position.z, z);
            rotation.yaw = calculate_relative_teleport(
                TeleportFlag::RelYaw,
                flags,
                rotation.yaw,
                -yaw as f64 * (PI / 180.0),
            );

            rotation.pitch = -((calculate_relative_teleport(
                TeleportFlag::RelPitch,
                flags,
                (-rotation.pitch) * (180.0 / PI) + 180.0,
                pitch,
            ) - 180.0)
                * (PI / 180.0));

            if (flags & (TeleportFlag::RelX as u8)) == 0 {
                velocity.velocity.x = 0.0;
            }
            if (flags & (TeleportFlag::RelY as u8)) == 0 {
                velocity.velocity.y = 0.0;
            }
            if (flags & (TeleportFlag::RelZ as u8)) == 0 {
                velocity.velocity.z = 0.0;
            }

            if let Some(teleport_id) = teleport_id {
                self.write_packet(packet::play::serverbound::TeleportConfirm { teleport_id });
            }
        }
    }

    fn on_block_entity_update(&self, block_update: packet::play::clientbound::UpdateBlockEntity) {
        match block_update.nbt {
            None => {
                // NBT is null, so we need to remove the block entity
                self.world
                    .clone()
                    .add_block_entity_action(world::BlockEntityAction::Remove(
                        block_update.location,
                    ));
            }
            Some(nbt) => {
                match block_update.action {
                    // TODO: support more block update actions
                    //1 => // Mob spawner
                    //2 => // Command block text
                    //3 => // Beacon
                    //4 => // Mob head
                    //5 => // Conduit
                    //6 => // Banner
                    //7 => // Structure
                    //8 => // Gateway
                    9 => {
                        // Sign
                        let line1 = format::Component::from_string(
                            nbt.1.get("Text1").unwrap().as_str().unwrap(),
                        );
                        let line2 = format::Component::from_string(
                            nbt.1.get("Text2").unwrap().as_str().unwrap(),
                        );
                        let line3 = format::Component::from_string(
                            nbt.1.get("Text3").unwrap().as_str().unwrap(),
                        );
                        let line4 = format::Component::from_string(
                            nbt.1.get("Text4").unwrap().as_str().unwrap(),
                        );
                        self.world.clone().add_block_entity_action(
                            world::BlockEntityAction::UpdateSignText(Box::new((
                                block_update.location,
                                line1,
                                line2,
                                line3,
                                line4,
                            ))),
                        );
                    }
                    //10 => // Unused
                    //11 => // Jigsaw
                    //12 => // Campfire
                    //14 => // Beehive
                    _ => {
                        debug!("Unsupported block entity action: {}", block_update.action);
                    }
                }
            }
        }
    }

    fn on_block_entity_update_data(
        &self,
        _block_update: packet::play::clientbound::UpdateBlockEntity_Data,
    ) {
        // TODO: handle UpdateBlockEntity_Data for 1.7, decompress gzipped_nbt
    }

    fn on_sign_update(&self, mut update_sign: packet::play::clientbound::UpdateSign) {
        format::convert_legacy(&mut update_sign.line1);
        format::convert_legacy(&mut update_sign.line2);
        format::convert_legacy(&mut update_sign.line3);
        format::convert_legacy(&mut update_sign.line4);
        self.world
            .clone()
            .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                update_sign.location,
                update_sign.line1,
                update_sign.line2,
                update_sign.line3,
                update_sign.line4,
            ))));
    }

    fn on_sign_update_u16(&self, mut update_sign: packet::play::clientbound::UpdateSign_u16) {
        format::convert_legacy(&mut update_sign.line1);
        format::convert_legacy(&mut update_sign.line2);
        format::convert_legacy(&mut update_sign.line3);
        format::convert_legacy(&mut update_sign.line4);
        self.world
            .clone()
            .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                Position::new(update_sign.x, update_sign.y as i32, update_sign.z),
                update_sign.line1,
                update_sign.line2,
                update_sign.line3,
                update_sign.line4,
            ))));
    }

    fn on_player_info_string(&self, _player_info: packet::play::clientbound::PlayerInfo_String) {
        // TODO: track online players, for 1.7.10 - this is for the <tab> online player list
        // self.players in 1.7.10 will be only spawned players (within client range)
        /*
        if player_info.online {
            self.players.entry(uuid.clone()).or_insert(PlayerInfo {
                name: player_info.name.clone(),
                uuid,
                skin_url: None,

                display_name: None,
                ping: player_info.ping as i32,
                gamemode: Gamemode::from_int(0),
            });
        } else {
            self.players.remove(&uuid);
        }
        */
    }

    fn on_player_info(&self, player_info: packet::play::clientbound::PlayerInfo) {
        use crate::protocol::packet::PlayerDetail::*;
        for detail in player_info.inner.players {
            match detail {
                Add {
                    name,
                    uuid,
                    properties,
                    display,
                    gamemode,
                    ping,
                } => {
                    let players = self.players.clone();
                    let mut players = players.write();
                    let info = players.entry(uuid.clone()).or_insert(PlayerInfo {
                        name: name.clone(),
                        uuid,
                        skin_url: None,

                        display_name: display.clone(),
                        ping: ping.0,
                        gamemode: GameMode::from_int(gamemode.0),
                    });
                    // Re-set the props of the player in case of dodgy server implementations
                    info.name = name;
                    info.display_name = display;
                    info.ping = ping.0;
                    info.gamemode = GameMode::from_int(gamemode.0);
                    for prop in properties {
                        if prop.name != "textures" {
                            continue;
                        }
                        // Ideally we would check the signature of the blob to
                        // verify it was from Mojang and not faked by the server
                        // but this requires the public key which is distributed
                        // authlib. We could download authlib on startup and extract
                        // the key but this seems like overkill compared to just
                        // whitelisting Mojang's texture servers instead.
                        let skin_blob_result = &base64::decode(&prop.value);
                        let skin_blob = match skin_blob_result {
                            Ok(val) => val,
                            Err(err) => {
                                error!("Failed to decode skin blob, {:?}", err);
                                continue;
                            }
                        };
                        let skin_blob: serde_json::Value = match serde_json::from_slice(skin_blob) {
                            Ok(val) => val,
                            Err(err) => {
                                error!("Failed to parse skin blob, {:?}", err);
                                continue;
                            }
                        };
                        if let Some(skin_url) = skin_blob
                            .pointer("/textures/SKIN/url")
                            .and_then(|v| v.as_str())
                        {
                            info.skin_url = Some(skin_url.to_owned());
                        }
                    }

                    // Refresh our own skin when the server sends it to us.
                    // The join game packet can come before this packet meaning
                    // we may not have the skin in time for spawning ourselves.
                    // This isn't an issue for other players because this packet
                    // must come before the spawn player packet.
                    if info.uuid == self.uuid {
                        let model = self
                            .entities
                            .clone()
                            .write()
                            .get_component_mut_direct::<entity::player::PlayerModel>(
                                self.player.clone().write().unwrap(),
                            )
                            .unwrap();
                        model.set_skin(info.skin_url.clone());
                    }
                }
                UpdateGamemode { uuid, gamemode } => {
                    if let Some(info) = self.players.clone().write().get_mut(&uuid) {
                        info.gamemode = GameMode::from_int(gamemode.0);
                    }
                }
                UpdateLatency { uuid, ping } => {
                    if let Some(info) = self.players.clone().write().get_mut(&uuid) {
                        info.ping = ping.0;
                    }
                }
                UpdateDisplayName { uuid, display } => {
                    if let Some(info) = self.players.clone().write().get_mut(&uuid) {
                        info.display_name = display;
                    }
                }
                Remove { uuid } => {
                    self.players.clone().write().remove(&uuid);
                }
            }
        }
    }

    fn on_servermessage_noposition(&self, m: packet::play::clientbound::ServerMessage_NoPosition) {
        self.on_servermessage(&m.message, None, None);
    }

    fn on_servermessage_position(&self, m: packet::play::clientbound::ServerMessage_Position) {
        self.on_servermessage(&m.message, Some(m.position), None);
    }

    fn on_servermessage_sender(&self, m: packet::play::clientbound::ServerMessage_Sender) {
        self.on_servermessage(&m.message, Some(m.position), Some(m.sender));
    }

    fn on_servermessage(
        &self,
        message: &format::Component,
        _position: Option<u8>,
        _sender: Option<protocol::UUID>,
    ) {
        debug!("Received chat message: {}", message);
        self.hud_context
            .clone()
            .write()
            .display_message_in_chat(message.clone());
        self.received_chat_at
            .clone()
            .write()
            .replace(Instant::now());
    }

    fn load_block_entities(&self, block_entities: Vec<Option<crate::nbt::NamedTag>>) {
        for block_entity in block_entities.into_iter().flatten() {
            let x = block_entity.1.get("x").unwrap().as_int().unwrap();
            let y = block_entity.1.get("y").unwrap().as_int().unwrap();
            let z = block_entity.1.get("z").unwrap().as_int().unwrap();
            if let Some(tile_id) = block_entity.1.get("id") {
                let tile_id = tile_id.as_str().unwrap();
                let action;
                match tile_id {
                    // Fake a sign update
                    "Sign" => action = 9,
                    // Not something we care about, so break the loop
                    _ => continue,
                }
                self.on_block_entity_update(packet::play::clientbound::UpdateBlockEntity {
                    location: Position::new(x, y, z),
                    action,
                    nbt: Some(block_entity.clone()),
                });
            } else {
                /*
                warn!(
                    "Block entity at ({},{},{}) missing id tag: {:?}",
                    x, y, z, block_entity
                );*/
            }
        }
    }

    fn on_chunk_data_biomes3d_varint(
        &self,
        chunk_data: packet::play::clientbound::ChunkData_Biomes3D_VarInt,
    ) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }

    fn on_chunk_data_biomes3d_bool(
        &self,
        chunk_data: packet::play::clientbound::ChunkData_Biomes3D_bool,
    ) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }

    fn on_chunk_data_biomes3d(&self, chunk_data: packet::play::clientbound::ChunkData_Biomes3D) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }

    fn on_chunk_data(&self, chunk_data: packet::play::clientbound::ChunkData) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }

    fn on_chunk_data_heightmap(&self, chunk_data: packet::play::clientbound::ChunkData_HeightMap) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }

    fn on_chunk_data_no_entities(
        &self,
        chunk_data: packet::play::clientbound::ChunkData_NoEntities,
    ) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
    }

    fn on_chunk_data_no_entities_u16(
        &self,
        chunk_data: packet::play::clientbound::ChunkData_NoEntities_u16,
    ) {
        let chunk_meta = vec![crate::protocol::packet::ChunkMeta {
            x: chunk_data.chunk_x,
            z: chunk_data.chunk_z,
            bitmask: chunk_data.bitmask,
        }];
        let skylight = false;
        self.world
            .clone()
            .load_chunks18(chunk_data.new, skylight, &chunk_meta, chunk_data.data.data)
            .unwrap();
    }

    fn on_chunk_data_17(&self, chunk_data: packet::play::clientbound::ChunkData_17) {
        self.world
            .clone()
            .load_chunk17(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask,
                chunk_data.add_bitmask,
                chunk_data.compressed_data.data,
            )
            .unwrap();
    }

    fn on_chunk_data_bulk(&self, bulk: packet::play::clientbound::ChunkDataBulk) {
        let new = true;
        self.world
            .clone()
            .load_chunks18(
                new,
                bulk.skylight,
                &bulk.chunk_meta.data,
                bulk.chunk_data.to_vec(),
            )
            .unwrap();
    }

    fn on_chunk_data_bulk_17(&self, bulk: packet::play::clientbound::ChunkDataBulk_17) {
        self.world
            .clone()
            .load_chunks17(
                bulk.chunk_column_count,
                bulk.data_length,
                bulk.skylight,
                &bulk.chunk_data_and_meta,
            )
            .unwrap();
    }

    fn on_chunk_unload(&self, chunk_unload: packet::play::clientbound::ChunkUnload) {
        self.world.clone().unload_chunk(
            chunk_unload.x,
            chunk_unload.z,
            &mut self.entities.clone().write(),
        );
    }

    fn on_block_change(&self, location: Position, id: i32) {
        self.on_block_change_in_world(location, id);
    }

    fn on_block_change_in_world(&self, location: Position, id: i32) {
        let world = self.world.clone();
        let modded_block_ids = world.modded_block_ids.clone();
        let block = world.id_map.by_vanilla_id(id as usize, modded_block_ids);
        world.set_block(location, block)
    }

    fn on_block_change_varint(&self, block_change: packet::play::clientbound::BlockChange_VarInt) {
        self.on_block_change(block_change.location, block_change.block_id.0)
    }

    fn on_block_change_u8(&self, block_change: packet::play::clientbound::BlockChange_u8) {
        self.on_block_change(
            crate::shared::Position::new(block_change.x, block_change.y as i32, block_change.z),
            (block_change.block_id.0 << 4) | (block_change.block_metadata as i32),
        );
    }

    fn on_multi_block_change_packed(
        &self,
        block_change: packet::play::clientbound::MultiBlockChange_Packed,
    ) {
        let sx = (block_change.chunk_section_pos >> 42) as i32;
        let sy = ((block_change.chunk_section_pos << 44) >> 44) as i32;
        let sz = ((block_change.chunk_section_pos << 22) >> 42) as i32;

        for record in block_change.records.data {
            let block_raw_id = record.0 >> 12;
            let lz = (record.0 & 0xf) as i32;
            let ly = ((record.0 >> 4) & 0xf) as i32;
            let lx = ((record.0 >> 8) & 0xf) as i32;

            /*
            let modded_block_ids = &self.world.clone().read().modded_block_ids;
            let block = self.world.clone().read()
                .id_map
                .by_vanilla_id(block_raw_id as usize, modded_block_ids);
            self.world.clone().write().set_block(
                Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32),
                block,
            );*/
            self.on_block_change(
                Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32),
                block_raw_id as i32,
            );
        }
    }

    fn on_multi_block_change_varint(
        &self,
        block_change: packet::play::clientbound::MultiBlockChange_VarInt,
    ) {
        let ox = block_change.chunk_x << 4;
        let oz = block_change.chunk_z << 4;
        for record in block_change.records.data {
            /*let modded_block_ids = &self.world.clone().read().modded_block_ids;
            let block = self.world.clone().read()
                .id_map
                .by_vanilla_id(record.block_id.0 as usize, modded_block_ids);
            self.world.clone().write().set_block(
                Position::new(
                    ox + (record.xz >> 4) as i32,
                    record.y as i32,
                    oz + (record.xz & 0xF) as i32,
                ),
                block,
            );*/
            self.on_block_change(
                Position::new(
                    ox + (record.xz >> 4) as i32,
                    record.y as i32,
                    oz + (record.xz & 0xF) as i32,
                ),
                record.block_id.0 as i32,
            );
        }
    }

    fn on_multi_block_change_u16(
        &self,
        block_change: packet::play::clientbound::MultiBlockChange_u16,
    ) {
        let ox = block_change.chunk_x << 4;
        let oz = block_change.chunk_z << 4;

        let mut data = std::io::Cursor::new(block_change.data);

        for _ in 0..block_change.record_count {
            use byteorder::{BigEndian, ReadBytesExt};

            let record = data.read_u32::<BigEndian>().unwrap();

            let id = record & 0x0000_ffff;
            let y = ((record & 0x00ff_0000) >> 16) as i32;
            let z = oz + ((record & 0x0f00_0000) >> 24) as i32;
            let x = ox + ((record & 0xf000_0000) >> 28) as i32;

            /*
            let modded_block_ids = &self.world.clone().read().modded_block_ids;
            let block = self.world.clone().read()
                .id_map
                .by_vanilla_id(id as usize, modded_block_ids);
            self.world.write().set_block(
                Position::new(x, y, z),
                block,
            );*/
            self.on_block_change(Position::new(x, y, z), id as i32);
        }
    }

    pub fn on_update_health(&self, health: f32, food: u8, saturation: u8) {
        self.hud_context
            .clone()
            .write()
            .update_health_and_food(health, food, saturation);
        if health <= 0.0 && !*self.dead.read() {
            *self.dead.write() = true;
            *self.just_died.write() = true;
        }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
enum TeleportFlag {
    RelX = 0b00001,
    RelY = 0b00010,
    RelZ = 0b00100,
    RelYaw = 0b01000,
    RelPitch = 0b10000,
}

fn calculate_relative_teleport(flag: TeleportFlag, flags: u8, base: f64, val: f64) -> f64 {
    if (flags & (flag as u8)) == 0 {
        val
    } else {
        base + val
    }
}
