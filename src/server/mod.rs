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
use crate::entity::player::{create_local, PlayerMovement, PlayerModel, CleanupManager};
use crate::entity::{EntityType, TargetRotation, GameInfo, Gravity, TargetPosition};
use crate::format;
use crate::inventory::material::versions::to_material;
use crate::inventory::{inventory_from_type, Inventory, InventoryContext, InventoryType, Item};
use crate::protocol::{self, forge, mapped_packet, packet};
use crate::render;
use crate::render::hud::HudContext;
use crate::render::Renderer;
use crate::resources;
use crate::screen::chat::{Chat, ChatContext};
use crate::screen::respawn::Respawn;
use crate::screen::ScreenSystem;
use crate::settings::Actionkey;
use crate::shared::Position;
use crate::types::hash::FNVHash;
use crate::types::GameMode;
use crate::world;
use crate::world::{CPos, LightData, LightUpdate};
use crate::{ecs, Game};
use cgmath::prelude::*;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use instant::{Duration, Instant};
use leafish_protocol::format::{Component, TextComponent};
use leafish_protocol::item::Stack;
use leafish_protocol::protocol::login::Account;
use leafish_protocol::protocol::mapped_packet::MappablePacket;
use leafish_protocol::protocol::mapped_packet::MappedPacket;
use leafish_protocol::protocol::packet::{DigType, Hand};
use leafish_protocol::protocol::{Conn, Version};
use leafish_shared::direction::Direction as BlockDirection;
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use parking_lot::RwLock;
use rand::{self, Rng};
use rayon::ThreadPoolBuilder;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::io::Cursor;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use crate::particle::block_break_effect::{BlockBreakEffect, BlockEffectData};
use crate::particle::ParticleType;
use cgmath::Vector3;
use bevy_ecs::prelude::{Entity, SystemStage, Stage};
use crate::ecs::{Manager, SystemExecStage};
use bevy_ecs::schedule::{StageLabel, DynHash, DynEq};

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
        Self {
            world_age: 0,
            world_time: 0.0,
            world_time_target: 0.0,
            tick_time: true,
        }
    }
}

pub struct Server {
    uuid: protocol::UUID,
    pub conn: Arc<RwLock<Option<protocol::Conn>>>,
    pub protocol_version: i32,
    pub mapped_protocol_version: Version,
    forge_mods: Vec<forge::ForgeMod>,
    pub disconnect_data: Arc<RwLock<DisconnectData>>,

    pub world: Arc<world::World>,
    pub entities: Arc<RwLock<ecs::Manager>>,
    world_data: Arc<RwLock<WorldData>>,

    resources: Arc<RwLock<resources::Manager>>,
    version: AtomicUsize,

    // Entity accessors
    block_break_info: Mutex<BlockBreakInfo>,
    //
    pub player: Arc<RwLock<Option<Entity>>>,
    entity_map: Arc<RwLock<HashMap<i32, Entity, BuildHasherDefault<FNVHash>>>>,
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
    last_chat_open: AtomicBool,
    pub chat_open: AtomicBool,
    pub chat_ctx: Arc<ChatContext>,
    screen_sys: Arc<ScreenSystem>,
    renderer: Arc<RwLock<Renderer>>,
    active_block_break_anims: Arc<DashMap<i32, Entity>>,
}

#[derive(Debug)]
pub struct BlockBreakInfo {
    break_position: Position,
    break_face: BlockDirection,
    hardness: f32,
    progress: f32,
    delay: u8,
    active: bool,
    pressed: bool,
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
        account: &Account,
        address: &str,
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        fml_network_version: Option<i64>,
        renderer: Arc<RwLock<Renderer>>,
        hud_context: Arc<RwLock<HudContext>>,
        screen_sys: Arc<ScreenSystem>,
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
            username: account.name.clone(),
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
                        screen_sys,
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
                        screen_sys,
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

        account.join_server(&server_id, &shared, &public_key)?;

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
            screen_sys,
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
        screen_sys: Arc<ScreenSystem>,
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
            screen_sys,
            renderer,
        ));
        server.hud_context.clone().write().server = Some(server.clone());

        let actual_server = server.clone();
        inner_server.replace(actual_server);
        render_list_computer.0.send(true).unwrap();
        server
    }

    fn spawn_reader(mut read: protocol::Conn, server: Arc<Mutex<Option<Arc<Server>>>>) {
        thread::spawn(move || {
            let threads = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
            loop {
                let server = server.clone().lock().as_ref().unwrap().clone();
                let pck = read.read_packet();
                match pck {
                    Ok(pck) => match pck.map() {
                        MappedPacket::KeepAliveClientbound(keep_alive) => packet::send_keep_alive(
                            server.conn.clone().write().as_mut().unwrap(),
                            server.mapped_protocol_version,
                            keep_alive.id,
                        )
                        .unwrap(),
                        MappedPacket::ChunkData_NoEntities(chunk_data) => {
                            server.on_chunk_data_no_entities(chunk_data);
                        }
                        MappedPacket::ChunkData_NoEntities_u16(chunk_data) => {
                            server.on_chunk_data_no_entities_u16(chunk_data);
                        }
                        MappedPacket::ChunkData_17(chunk_data) => {
                            server.on_chunk_data_17(chunk_data);
                        }
                        MappedPacket::ChunkDataBulk(bulk) => {
                            server.on_chunk_data_bulk(bulk);
                        }
                        MappedPacket::ChunkDataBulk_17(bulk) => {
                            server.on_chunk_data_bulk_17(bulk);
                        }
                        MappedPacket::BlockChange(block_change) => {
                            server.on_block_change(block_change);
                        }
                        MappedPacket::MultiBlockChange(block_change) => {
                            server.on_multi_block_change(block_change);
                        }
                        MappedPacket::UpdateBlockEntity(block_update) => {
                            server.on_block_entity_update(block_update);
                        }
                        MappedPacket::ChunkData_Biomes3D(chunk_data) => {
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d(chunk_data);
                            });
                        }
                        MappedPacket::ChunkData_Biomes3D_i32(chunk_data) => {
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d_varint(chunk_data);
                            });
                        }
                        MappedPacket::ChunkData_Biomes3D_bool(chunk_data) => {
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d_bool(chunk_data);
                            });
                        }
                        MappedPacket::ChunkData(chunk_data) => {
                            threads.spawn(move || {
                                server.on_chunk_data(chunk_data);
                            });
                        }
                        MappedPacket::ChunkData_HeightMap(chunk_data) => {
                            threads.spawn(move || {
                                server.on_chunk_data_heightmap(chunk_data);
                            });
                        }
                        MappedPacket::UpdateSign(update_sign) => {
                            server.on_sign_update(update_sign);
                        }
                        /*
                        MappedPacket::UpdateBlockEntity_Data(block_update) => {
                            server.on_block_entity_update_data(block_update);
                        }*/
                        MappedPacket::ChunkUnload(chunk_unload) => {
                            server.on_chunk_unload(chunk_unload);
                        }
                        MappedPacket::EntityDestroy(entity_destroy) => {
                            server.on_entity_destroy(entity_destroy);
                        }
                        MappedPacket::EntityMove(m) => {
                            server.on_entity_move(m);
                        }
                        MappedPacket::EntityLook(look) => {
                            server.on_entity_look(
                                look.entity_id,
                                look.yaw as f64,
                                look.pitch as f64,
                            );
                        }
                        MappedPacket::EntityHeadLook(look) => {
                            use std::f64::consts::PI;
                            if let Some(entity) =
                                server.entity_map.clone().read().get(&look.entity_id)
                            {
                                let entities = server.entities.clone();
                                let mut entities = entities.write();
                                let mut rotation = entities
                                    .world.get_entity_mut(*entity).unwrap().get_mut::<TargetRotation>().unwrap();
                                rotation.yaw = -(look.head_yaw as f64 / 256.0) * PI * 2.0;
                            }
                        }
                        MappedPacket::JoinGame(join) => {
                            server.on_game_join(join.gamemode, join.entity_id);
                        }
                        MappedPacket::TeleportPlayer(teleport) => {
                            server.on_teleport_player(teleport);
                        }
                        MappedPacket::Respawn(respawn) => {
                            server.on_respawn(respawn);
                        }
                        MappedPacket::SpawnMob(spawn) => {
                            use std::f64::consts::PI;
                            server.on_entity_spawn(
                                spawn.ty as i16,
                                spawn.entity_id,
                                spawn.x,
                                spawn.y,
                                spawn.z,
                                -(spawn.yaw as f64 / 256.0) * PI * 2.0,
                                -(spawn.pitch as f64 / 256.0) * PI * 2.0,
                            );
                        }
                        MappedPacket::SpawnObject(spawn) => {
                            use std::f64::consts::PI;
                            server.on_entity_spawn(
                                spawn.ty as i16,
                                spawn.entity_id,
                                spawn.x,
                                spawn.y,
                                spawn.z,
                                -(spawn.yaw as f64 / 256.0) * PI * 2.0,
                                -(spawn.pitch as f64 / 256.0) * PI * 2.0,
                            );
                        }
                        MappedPacket::EntityTeleport(entity_teleport) => {
                            server.on_entity_teleport(
                                entity_teleport.entity_id,
                                entity_teleport.x,
                                entity_teleport.y,
                                entity_teleport.z,
                                entity_teleport.yaw as f64,
                                entity_teleport.pitch as f64,
                                entity_teleport.on_ground.unwrap_or(true), // TODO: how is this default supposed to be set? (for 1.7)
                            );
                        }
                        MappedPacket::EntityLookAndMove(lookmove) => {
                            server.on_entity_look_and_move(
                                lookmove.entity_id,
                                lookmove.delta_x,
                                lookmove.delta_y,
                                lookmove.delta_z,
                                lookmove.yaw as f64,
                                lookmove.pitch as f64,
                            );
                        }
                        MappedPacket::SpawnPlayer(spawn) => {
                            if spawn.uuid_str.is_some() {
                                // 1.7.10: populate the player list here, since we only now know the UUID
                                let uuid =
                                    protocol::UUID::from_str(spawn.uuid_str.as_ref().unwrap())
                                        .unwrap();
                                server
                                    .players
                                    .clone()
                                    .write()
                                    .entry(uuid.clone())
                                    .or_insert(PlayerInfo {
                                        name: spawn.name.unwrap().clone(),
                                        uuid,
                                        skin_url: None,
                                        display_name: None,
                                        ping: 0, // TODO: don't overwrite from PlayerInfo_String
                                        gamemode: GameMode::from_int(0),
                                    });
                            }
                            let uuid = if spawn.uuid_str.is_some() {
                                protocol::UUID::from_str(spawn.uuid_str.as_ref().unwrap()).unwrap()
                            } else {
                                spawn.uuid.unwrap()
                            };
                            server.on_player_spawn(
                                spawn.entity_id,
                                uuid,
                                spawn.x,
                                spawn.y,
                                spawn.z,
                                spawn.pitch as f64,
                                spawn.yaw as f64,
                            );
                        }
                        MappedPacket::PlayerInfo(player_info) => {
                            server.on_player_info(player_info);
                        }
                        MappedPacket::ConfirmTransaction(transaction) => {
                            read.write_packet(
                                packet::play::serverbound::ConfirmTransactionServerbound {
                                    id: 0, // TODO: Use current container id, if the id of the transaction is not 0.
                                    action_number: transaction.action_number,
                                    accepted: true,
                                },
                            )
                            .unwrap();
                        }
                        MappedPacket::UpdateLight(update_light) => {
                            server.world.clone().lighting_cache.clone().write().insert(
                                CPos(update_light.chunk_x, update_light.chunk_z),
                                LightData {
                                    arrays: Cursor::new(update_light.light_arrays),
                                    block_light_mask: update_light.block_light_mask,
                                    sky_light_mask: update_light.sky_light_mask,
                                },
                            );
                        }
                        MappedPacket::ChangeGameState(game_state) => {
                            server.on_game_state_change(game_state);
                        }
                        MappedPacket::UpdateHealth(update_health) => {
                            server.on_update_health(
                                update_health.health,
                                update_health.food as u8,
                                update_health.food_saturation as u8,
                            );
                        }
                        MappedPacket::TimeUpdate(time_update) => {
                            server.on_time_update(time_update);
                        }
                        MappedPacket::Disconnect(disconnect) => {
                            server.disconnect(Some(disconnect.reason));
                        }
                        MappedPacket::ServerMessage(server_message) => {
                            server.on_servermessage(server_message);
                        }
                        MappedPacket::PlayerInfo_String(player_info) => {
                            server.on_player_info_string(player_info);
                        }
                        MappedPacket::PluginMessageClientbound(plugin_message) => {
                            server.on_plugin_message_clientbound(plugin_message);
                        }
                        MappedPacket::SetExperience(set_exp) => {
                            server
                                .hud_context
                                .clone()
                                .write()
                                .update_exp(set_exp.experience_bar, set_exp.level);
                        }
                        MappedPacket::SetCurrentHotbarSlot(set_slot) => {
                            if set_slot.slot <= 8 {
                                server.inventory_context.clone().write().hotbar_index =
                                    set_slot.slot;
                                server
                                    .hud_context
                                    .clone()
                                    .write()
                                    .update_slot_index(set_slot.slot);
                            } else {
                                warn!("The server tried to set the hotbar slot to {}, although it has to be in a range of 0-8! Did it try to crash you?", set_slot.slot);
                            }
                        }
                        MappedPacket::WindowItems(items) => {
                            for item in items.items.into_iter().enumerate() {
                                server.on_set_slot(items.id as i16, item.0 as i16, item.1);
                            }
                        }
                        MappedPacket::WindowSetSlot(set_slot) => {
                            server.on_set_slot(set_slot.id as i16, set_slot.slot, set_slot.item);
                        }
                        MappedPacket::WindowClose(_close) => {
                            server
                                .inventory_context
                                .clone()
                                .write()
                                .try_close_inventory(server.screen_sys.clone());
                        }
                        MappedPacket::WindowOpen(open) => {
                            if open.ty_name.is_some() {
                                print!("inv type name: {}", open.ty_name.as_ref().unwrap());
                            } else {
                                let inv_type = InventoryType::from_id(open.ty.unwrap());
                                let inventory = inventory_from_type(
                                    inv_type,
                                    open.title,
                                    server.renderer.clone(),
                                    server.hud_context.clone(),
                                    server.inventory_context.read().base_inventory.clone(),
                                    open.id,
                                );
                                if let Some(inventory) = inventory {
                                    server.inventory_context.clone().write().open_inventory(
                                        inventory.clone(),
                                        server.screen_sys.clone(),
                                        server.inventory_context.clone(),
                                    );
                                }
                            }
                        }
                        MappedPacket::EntityVelocity(_velocity) => {
                            // TODO: Only apply the velocity to the local player due to jittering of other players
                            /* if let Some(entity) =
                                server.entity_map.clone().read().get(&velocity.entity_id)
                            {
                                let entity_velocity = server
                                    .entities
                                    .clone()
                                    .write()
                                    .get_component_mut(*entity, server.velocity)
                                    .unwrap();
                                entity_velocity.velocity = Vector3::new(
                                    velocity.velocity_x as f64 / 8000.0,
                                    velocity.velocity_y as f64 / 8000.0,
                                    velocity.velocity_z as f64 / 8000.0,
                                );
                            }*/
                        }
                        MappedPacket::BlockBreakAnimation(block_break) => {
                            println!("Block_break_anim: {}", block_break.stage);
                            if block_break.stage >= 10 {
                                server.entities.clone().write().world.despawn(server.active_block_break_anims.remove(&block_break.entity_id).unwrap().1);
                            } else if let Some(anim) = server.active_block_break_anims.clone().get_mut(&block_break.entity_id) {
                                let entities = server.entities.clone();
                                let mut entities = entities.write();
                                let mut anim = entities.world.get_entity_mut(*anim.value()).unwrap();
                                anim.get_mut::<BlockBreakEffect>().unwrap().update(block_break.stage);
                            } else {
                                let entities = server.entities.clone();
                                let mut entities = entities.write();
                                let mut entity = entities.world.spawn();
                                entity.insert(BlockEffectData {
                                    position: Vector3::new(block_break.location.x as f64, block_break.location.y as f64, block_break.location.z as f64),
                                    status: block_break.stage,
                                });
                                let entity = entity.id();
                                let particle = ParticleType::BlockBreak.create_particle(&mut entities, entity);
                                server.active_block_break_anims.clone().insert(block_break.entity_id, particle.unwrap());
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
            }
        });
    }

    fn spawn_light_updater(_server: Arc<Mutex<Option<Arc<Server>>>>) -> Sender<LightUpdate> {
        let (tx, rx) = unbounded();
        thread::spawn(move || loop {
            /*let server = server.clone().lock().as_ref().unwrap().clone();
            let mut done = false; // TODO: Improve performance!
            while !done {
                let start = Instant::now();
                let mut updates_performed = 0;
                let world_cloned = server.world.clone();
                let mut interrupt = false;
                while let Ok(update) = rx.try_recv() {
                    updates_performed += 1;
                    world_cloned.do_light_update(update);
                    if (updates_performed & 0xFFF == 0) && start.elapsed().subsec_nanos() >= 5000000
                    {
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
           // println!("doing render list thingy!");
            let _: bool = rx.recv().unwrap();
            let server = server.clone().lock().as_ref().unwrap().clone();
            let world = server.world.clone();
            world.compute_render_list(renderer.clone());
            while rx.try_recv().is_ok() {}
            etx.send(true).unwrap();
        });
        (tx, erx)
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
        screen_sys: Arc<ScreenSystem>,
        renderer: Arc<RwLock<Renderer>>,
    ) -> Server {
        let world = Arc::new(world::World::new(protocol_version, light_updater));
        let mut entities = Manager::new();
        let mut parallel = SystemStage::parallel();
        let mut sync = SystemStage::single_threaded();
        entities.world.insert_resource(entity::GameInfo::new());
        entities.world.insert_resource(world.clone());
        entities.world.insert_resource(renderer.clone());
        entities.world.insert_resource(screen_sys.clone());
        entities.world.insert_resource(CleanupManager::default());
        entity::add_systems(&mut entities, &mut parallel, &mut sync);
        entities.schedule.clone().write().add_stage("parallel", parallel).add_stage_after("parallel", "sync", sync);

        let version = Version::from_id(protocol_version as u32);
        let inventory_context = Arc::new(RwLock::new(InventoryContext::new(
            version,
            &renderer.read(),
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

            world: world.clone(),
            world_data: Arc::new(RwLock::new(WorldData::default())),
            version: AtomicUsize::new(version),
            resources,

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
            block_break_info: Mutex::new(BlockBreakInfo {
                break_position: Default::default(),
                break_face: BlockDirection::Invalid,
                hardness: 0.0,
                progress: 0.0,
                delay: 0,
                active: false,
                pressed: false,
            }),
            last_chat_open: AtomicBool::new(false),
            chat_open: AtomicBool::new(false),
            chat_ctx: Arc::new(ChatContext::new()),
            screen_sys,
            renderer,
            active_block_break_anims: Arc::new(Default::default()),
        }
    }

    pub fn disconnect(&self, reason: Option<format::Component>) {
        self.conn.clone().write().take().unwrap().close();
        self.disconnect_data.clone().write().disconnect_reason = reason;
        if let Some(player) = self.player.clone().write().take() {
            self.entities.clone().write().world.despawn(player);
        }
        self.disconnect_data.clone().write().just_disconnected = true;
    }

    pub fn is_connected(&self) -> bool {
        return self.conn.clone().read().is_some();
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
        let chat_open = self.chat_open.load(Ordering::Acquire);
        if chat_open != self.last_chat_open.load(Ordering::Acquire) {
            self.last_chat_open.store(chat_open, Ordering::Release);
            if chat_open {
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(Chat::new(self.chat_ctx.clone())));
            } else {
                game.screen_sys.clone().pop_screen();
            }
        }
        let version = self.resources.read().version();
        if version != self.version.load(Ordering::Acquire) {
            self.version.store(version, Ordering::Release);
            self.world.clone().flag_dirty_all();
        }
        {
            let renderer = &mut renderer.write();
            // TODO: Check if the world type actually needs a sun
            if self.sun_model.read().is_none() {
                self.sun_model.write().replace(sun::SunModel::new(renderer));
            }

            // Copy to camera
            if let Some(player) = *self.player.clone().read() {
                let entities = self.entities.read();
                let position = entities
                    .world
                    .get_entity(player)
                    .unwrap().get::<crate::entity::Position>()
                    .unwrap();
                let rotation = entities
                    .world
                    .get_entity(player)
                    .unwrap().get::<crate::entity::Rotation>()
                    .unwrap();
                renderer.camera.pos = cgmath::Point3::from_vec(
                    position.position + cgmath::Vector3::new(0.0, 1.62, 0.0),
                );
                renderer.camera.yaw = rotation.yaw;
                renderer.camera.pitch = rotation.pitch;
            }
        }
        self.entity_tick(/*renderer, */delta, game.focused, *self.dead.read());

        *self.tick_timer.write() += delta;
        while *self.tick_timer.read() >= 3.0 && self.is_connected() {
            self.minecraft_tick(game);
            *self.tick_timer.write() -= 3.0;
        }
        let renderer = &mut renderer.write();

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
                game.screen_sys.close_closable_screens();
                game.screen_sys
                    .clone()
                    .add_screen(Box::new(Respawn::new(0))); // TODO: Use the correct score!
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

    fn entity_tick(&self, /*renderer: &mut render::Renderer, */delta: f64, focused: bool, dead: bool) {
        let entities = self.entities.clone();
        let mut entities = entities.write();
        {
            let mut game_info = entities.world.get_resource_mut::<GameInfo>().unwrap();
            // Update the game's state for entities to read
            game_info
                .delta = delta;
        }

        if self.is_connected() || self.disconnect_data.clone().read().just_disconnected {
            // Allow an extra tick when disconnected to clean up
            self.disconnect_data.clone().write().just_disconnected = false;
            *self.entity_tick_timer.write() += delta;
            while *self.entity_tick_timer.read() >= 3.0 {
                /*println!("pre run!");
                let schedule = entities.schedule.clone();
                println!("run!");
                schedule.write().run(&mut entities.world);*/
                *self.entity_tick_timer.write() -= 3.0;
            }
            // println!("pre run!");
            let schedule = entities.schedule.clone();
            // println!("run!");
            schedule.write().run(&mut entities.world);
            // println!("finished run!");
            // let world = self.world.clone();
            /*self.entities
                .clone()
                .write()
                .render_tick(&world, renderer, focused, dead);*/
            // TODO: Make render systems run only once!
        }
    }

    pub fn remove(&mut self, renderer: &mut render::Renderer) {
        let world = self.world.clone();
        // TODO: Handle remove of all entities if necessary!
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

    pub fn minecraft_tick(&self, game: &mut Game) {
        if let Some(player) = *self.player.clone().write() {
            let on_ground = {
                let entities = self.entities.clone();
                let mut entities = entities.write();
                let mut movement = entities
                    .world.entity_mut(player)
                    .get_mut::<PlayerMovement>().unwrap();
                // Force the server to know when touched the ground
                // otherwise if it happens between ticks the server
                // will think we are flying.
                if movement.did_touch_ground {
                    movement.did_touch_ground = false;
                    Some(true)
                } else {
                    None
                }
            }.unwrap_or_else(|| {
                self
                    .entities
                    .read()
                    .world.entity(player)
                    .get::<Gravity>()
                    .map_or(false, |v| v.on_ground)
            });
            let entities = self.entities.read();

            let position = entities
                .world.entity(player)
                .get::<TargetPosition>()
                .unwrap();
            let rotation = entities
                .world.entity(player)
                .get::<crate::entity::Rotation>()
                .unwrap();



            // Sync our position to the server
            // Use the smaller packets when possible
            packet::send_position_look(
                self.conn.clone().write().as_mut().unwrap(),
                self.mapped_protocol_version,
                &position.position,
                rotation.yaw as f32,
                rotation.pitch as f32,
                on_ground,
            )
            .unwrap();
        }
        if !game.focused {
            if self.block_break_info.lock().pressed {
                self.abort_breaking();
            }
            return;
        }
        let break_delay = self.block_break_info.lock().delay;
        if break_delay > 0 {
            self.block_break_info.lock().delay -= 1;
        } else if self.block_break_info.lock().active {
            if self.block_break_info.lock().progress >= 1.0 {
                let face_idx = self.block_break_info.lock().break_face.index() as u8;
                packet::send_digging(
                    self.conn.clone().write().as_mut().unwrap(),
                    self.mapped_protocol_version,
                    DigType::StopDestroyBlock,
                    self.block_break_info.lock().break_position,
                    face_idx,
                )
                .unwrap();
                self.block_break_info.lock().active = false;
                self.block_break_info.lock().delay = 5;
            } else {
                packet::send_arm_swing(
                    self.conn.clone().write().as_mut().unwrap(),
                    self.mapped_protocol_version,
                    Hand::MainHand,
                )
                .unwrap();
                self.block_break_info.lock().progress += 0.1; // TODO: Make this value meaningful
            }
        } else if self.block_break_info.lock().pressed {
            self.on_left_click();
        }
    }

    pub fn key_press(&self, down: bool, key: Actionkey, focused: &mut bool) -> bool {
        if *focused || key == Actionkey::OpenInv || key == Actionkey::ToggleChat {
            let mut state_changed = false;
            if let Some(player) = *self.player.clone().write() {
                if let Some(mut movement) = self
                    .entities
                    .clone()
                    .write()
                    .world.entity_mut(player)
                    .get_mut::<PlayerMovement>()
                {
                    state_changed = movement.pressed_keys.get(&key).map_or(false, |v| *v) != down;
                    movement.pressed_keys.insert(key, down);
                }
            }
            match key {
                Actionkey::OpenInv => {
                    if down {
                        let player_inv = self
                            .inventory_context
                            .clone()
                            .read()
                            .player_inventory
                            .clone();
                        self.inventory_context.clone().write().open_inventory(
                            player_inv,
                            self.screen_sys.clone(),
                            self.inventory_context.clone(),
                        );
                        return true;
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
                Actionkey::ToggleChat => {
                    if down {
                        self.screen_sys
                            .add_screen(Box::new(Chat::new(self.chat_ctx.clone())));
                        return true;
                    }
                }
                _ => {}
            };
        }
        false
    }

    pub fn on_left_click(&self) {
        packet::send_arm_swing(
            self.conn.clone().write().as_mut().unwrap(),
            self.mapped_protocol_version,
            Hand::MainHand,
        )
        .unwrap();
        // TODO: Implement clientside animation.
        if self.player.clone().read().is_some() {
            let world = self.world.clone();
            let gamemode = *self
                .entities
                .read()
                .world.entity(*self.player.clone().read().as_ref().unwrap())
                .get::<GameMode>()
                .unwrap();
            if gamemode.can_interact_with_world() && self.block_break_info.lock().delay == 0 {
                self.block_break_info.lock().pressed = true;
                // TODO: Check this
                if let Some((pos, _, face, _)) = target::trace_ray(
                    &world,
                    4.0,
                    self.renderer.read().camera.pos.to_vec(),
                    self.renderer.read().view_vector.cast().unwrap(),
                    target::test_block,
                ) {
                    packet::send_digging(
                        self.conn.clone().write().as_mut().unwrap(),
                        self.mapped_protocol_version,
                        DigType::StartDestroyBlock,
                        pos,
                        face.index() as u8,
                    )
                    .unwrap();
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
        if self.block_break_info.lock().pressed {
            self.block_break_info.lock().pressed = false;
        }
        // TODO: Call this on hand switching (hotbar scrolling), but let pressed as it is!
        if self.block_break_info.lock().active {
            self.block_break_info.lock().active = false;
            self.block_break_info.lock().delay = 5;
            let pos = self.block_break_info.lock().break_position;
            let face = self.block_break_info.lock().break_face;
            packet::send_digging(
                self.conn.clone().write().as_mut().unwrap(),
                self.mapped_protocol_version,
                DigType::AbortDestroyBlock,
                pos,
                face.index() as u8,
            )
            .unwrap();
        }
    }

    pub fn on_right_click(&self) {
        if self.player.clone().read().is_some() {
            let world = self.world.clone();
            let gamemode = *self
                .entities
                .read()
                .world.entity(*self.player.clone().read().as_ref().unwrap())
                .get::<GameMode>()
                .unwrap();
            if gamemode.can_interact_with_world() {
                // TODO: Check this
                if let Some((pos, _, face, at)) = target::trace_ray(
                    &world,
                    4.0,
                    self.renderer.read().camera.pos.to_vec(),
                    self.renderer.read().view_vector.cast().unwrap(),
                    target::test_block,
                ) {
                    let hud_context = self.hud_context.clone();
                    packet::send_block_place(
                        self.conn.clone().write().as_mut().unwrap(),
                        self.mapped_protocol_version,
                        pos,
                        face.index() as u8,
                        at,
                        Hand::MainHand,
                        Box::new(move || {
                            hud_context
                                .clone()
                                .read()
                                .player_inventory
                                .as_ref()
                                .unwrap()
                                .clone()
                                .read()
                                .get_item((36 + hud_context.clone().read().get_slot_index()) as u16)
                                .as_ref()
                                .map(|item| item.stack.clone())
                        }),
                    )
                    .unwrap();
                    packet::send_arm_swing(
                        self.conn.clone().write().as_mut().unwrap(),
                        self.mapped_protocol_version,
                        Hand::MainHand,
                    )
                    .unwrap();
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

    fn on_plugin_message_clientbound(
        &self,
        msg: mapped_packet::play::clientbound::PluginMessageClientbound,
    ) {
        if protocol::is_network_debug() {
            debug!(
                "Received plugin message: channel={}, data={:?}",
                msg.channel, msg.data
            );
        }

        match &*msg.channel {
            "REGISTER" => {}   // TODO
            "UNREGISTER" => {} // TODO
            "FML|HS" => {
                let msg =
                    crate::protocol::Serializable::read_from(&mut std::io::Cursor::new(msg.data))
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

    fn on_set_slot(&self, inventory_id: i16, slot: i16, item: Option<Stack>) {
        println!(
            "set item {:?} to slot {} to inv {}",
            item.as_ref(),
            slot,
            inventory_id
        );
        let top_inventory = self.inventory_context.clone();
        let inventory = if inventory_id == -1 || inventory_id == 0 {
            top_inventory.read().player_inventory.clone() // TODO: This caused a race condition, check why!
        } else if let Some(inventory) = top_inventory.read().safe_inventory.as_ref() {
            inventory.clone()
        } else {
            println!("Couldn't set item to slot {}", slot);
            return;
        };
        let curr_slots = inventory.clone().read().size();
        if slot < 0 || slot as u16 >= curr_slots {
            if slot == -1 {
                let item = item.map(|stack| {
                    let id = stack.id;
                    Item {
                        stack,
                        material: to_material(id as u16, self.mapped_protocol_version),
                    }
                });
                top_inventory.write().cursor = item; // TODO: Set to HUD and make it dirty!
            } else {
                warn!("The server tried to set an item to slot {} but the current inventory only has {} slots. Did it try to crash you?", slot, curr_slots);
            }
        } else {
            let item = item.map(|stack| {
                let id = stack.id;
                Item {
                    stack,
                    material: to_material(id as u16, self.mapped_protocol_version),
                }
            });
            inventory.clone().write().set_item(slot as u16, item);
        }
    }

    fn on_game_join(&self, gamemode: u8, entity_id: i32) {
        let gamemode = GameMode::from_int((gamemode & 0x7) as i32);
        let player = entity::player::create_local(&mut self.entities.clone().write());
        if let Some(info) = self.players.clone().read().get(&self.uuid) {
            let entities = self.entities.clone();
            let mut entities = entities.write();
            let mut model = entities
                .world.entity_mut(player)
                .get_mut::<PlayerModel>()
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.hud_context.clone().write().update_game_mode(gamemode);
        *self
            .entities
            .clone()
            .write()
            .world.entity_mut(player)
            .get_mut::<GameMode>()
            .unwrap() = gamemode;
        self.entities
            .clone()
            .write()
            .world.entity_mut(player).get_mut::<PlayerMovement>()
            .unwrap()
            .flying = gamemode.can_fly();

        self.entity_map.clone().write().insert(entity_id, player);
        self.player.clone().write().replace(player);

        // Let the server know who we are
        let brand = plugin_messages::Brand {
            brand: "leafish".into(),
        };
        brand.write_to(self.conn.clone().write().as_mut().unwrap());

        packet::send_client_settings(
            self.conn.clone().write().as_mut().unwrap(),
            self.mapped_protocol_version,
            "en_us".to_string(),
            8,
            0,
            true,
            127,
            Hand::MainHand,
        )
        .unwrap(); // TODO: Make these configurable!
    }

    fn on_respawn(&self, respawn: mapped_packet::play::clientbound::Respawn) {
        let gamemode = GameMode::from_int((respawn.gamemode & 0x7) as i32);

        if let Some(player) = *self.player.clone().write() {
            self.hud_context.clone().write().update_game_mode(gamemode);
            *self
                .entities
                .clone()
                .write()
                .world.entity_mut(player)
                .get_mut::<GameMode>()
                .unwrap() = gamemode;
            self.entities
                .clone()
                .write()
                .world.entity_mut(player)
                .get_mut::<PlayerMovement>()
                .unwrap()
                .flying = gamemode.can_fly();
        }
        // TODO: Handle remove of all entities (gracefully) if necessary!
        *self.player.clone().write() = Some(create_local(&mut *self.entities.clone().write()));
        if *self.dead.read() {
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
            self.screen_sys.pop_screen();
        }
    }

    // TODO: make use of "on_disconnect"
    #[allow(dead_code)]
    fn on_disconnect(&self, disconnect: packet::play::clientbound::Disconnect) {
        self.disconnect(Some(disconnect.reason));
    }

    fn on_time_update(&self, time_update: mapped_packet::play::clientbound::TimeUpdate) {
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

    fn on_game_state_change(&self, game_state: mapped_packet::play::clientbound::ChangeGameState) {
        if game_state.reason == 3 {
            if let Some(player) = *self.player.write() {
                let gamemode = GameMode::from_int(game_state.value as i32);
                self.hud_context.clone().write().update_game_mode(gamemode);
                *self
                    .entities
                    .clone()
                    .write()
                    .world.entity_mut(player)
                    .get_mut::<GameMode>()
                    .unwrap() = gamemode;
                self.entities
                    .clone()
                    .write()
                    .world.entity_mut(player)
                    .get_mut::<PlayerMovement>()
                    .unwrap()
                    .flying = gamemode.can_fly();
            }
        }
    }

    fn on_entity_spawn(
        &self,
        ty: i16,
        entity_id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
    ) {
        let entity_type = entity::versions::to_entity_type(ty, self.mapped_protocol_version);
        if entity_type != EntityType::Unknown {
            let entity =
                entity_type.create_entity(&mut self.entities.clone().write(), x, y, z, yaw, pitch);
            if let Some(entity) = entity {
                self.entity_map.clone().write().insert(entity_id, entity);
                println!("spawned {} {:?}", ty, entity_type);
            }
        }
    }

    fn on_entity_destroy(&self, entity_destroy: mapped_packet::play::clientbound::EntityDestroy) {
        for id in entity_destroy.entity_ids {
            if let Some(entity) = self.entity_map.clone().write().remove(&id) {
                self.entities.clone().write().world.despawn(entity);
            }
        }
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
            {
                let entities = self.entities.clone();
                let mut entities = entities.write();
                let mut target_position = entities
                    .world.entity_mut(*entity)
                    .get_mut::<TargetPosition>()
                    .unwrap();
                target_position.position.x = x;
                target_position.position.y = y;
                target_position.position.z = z;
            }
            let entities = self.entities.clone();
            let mut entities = entities.write();
            let mut target_rotation = entities
                .world.entity_mut(*entity)
                .get_mut::<TargetRotation>()
                .unwrap();
            target_rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            target_rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_move(&self, entity_move: mapped_packet::play::clientbound::EntityMove) {
        if let Some(entity) = self.entity_map.clone().read().get(&entity_move.entity_id) {
            let entities = self.entities.clone();
            let mut entities = entities.write();
            let mut position = entities
                .world.entity_mut(*entity)
                .get_mut::<TargetPosition>()
                .unwrap();
            position.position.x += entity_move.delta_x;
            position.position.y += entity_move.delta_y;
            position.position.z += entity_move.delta_z;
        }
    }

    fn on_entity_look(&self, entity_id: i32, yaw: f64, pitch: f64) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().get(&entity_id) {
            let entities = self.entities.clone();
            let mut entities = entities.write();
            let mut rotation = entities
                .world.entity_mut(*entity)
                .get_mut::<TargetRotation>()
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
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
            {
                let entities = self.entities.clone();
                let mut entities = entities.write();
            let mut position = entities
                .world.entity_mut(*entity)
                .get_mut::<TargetPosition>()
                .unwrap();
                position.position.x += delta_x;
                position.position.y += delta_y;
                position.position.z += delta_z;
            }
            let entities = self.entities.clone();
            let mut entities = entities.write();
            let mut rotation = entities
                .world.entity_mut(*entity)
                .get_mut::<TargetRotation>()
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
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
            self.entities.clone().write().world.despawn(entity);
        }
        let entity = entity::player::create_remote(
            &mut self.entities.clone().write(),
            self.players
                .clone()
                .read()
                .get(&uuid)
                .map_or("MISSING", |v| &v.name),
        );
        let entities = self.entities.clone();
        let mut entities = entities.write();
        {
            let mut position = entities
                .world.entity_mut(entity)
                .get_mut::<crate::entity::Position>()
                .unwrap();
            position.position.x = x;
            position.position.y = y;
            position.position.z = z;
        }
        {
            let mut target_position = entities
                .world.entity_mut(entity)
                .get_mut::<TargetPosition>()
                .unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
        }
        let (yaw, pitch) = {
            let mut rotation = entities
                .world.entity_mut(entity)
                .get_mut::<crate::entity::Rotation>()
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
            (rotation.yaw, rotation.pitch)
        };
        {
        let mut target_rotation = entities
            .world.entity_mut(entity)
            .get_mut::<TargetRotation>()
            .unwrap();
        target_rotation.yaw = yaw;
        target_rotation.pitch = pitch;
        }
        if let Some(info) = self.players.clone().read().get(&uuid) {
            let mut model = entities
                .world.entity_mut(entity)
                .get_mut::<PlayerModel>()
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.entity_map.clone().write().insert(entity_id, entity);
    }

    fn on_teleport_player(&self, teleport: mapped_packet::play::clientbound::TeleportPlayer) {
        use std::f64::consts::PI;
        if let Some(player) = *self.player.clone().write() {
            let flags = teleport.flags.unwrap_or(0);
            let entities = self.entities.clone();
            let mut entities = entities.write();

            {
                let mut position = entities
                    .world.entity_mut(player)
                    .get_mut::<TargetPosition>()
                    .unwrap();
                position.position.x = calculate_relative_teleport(
                    TeleportFlag::RelX,
                    flags,
                    position.position.x,
                    teleport.x,
                );
                position.position.y = calculate_relative_teleport(
                    TeleportFlag::RelY,
                    flags,
                    position.position.y,
                    teleport.y,
                );
                position.position.z = calculate_relative_teleport(
                    TeleportFlag::RelZ,
                    flags,
                    position.position.z,
                    teleport.z,
                );
            }
            {
            let mut rotation = entities
                .world.entity_mut(player)
                .get_mut::<crate::entity::Rotation>()
                .unwrap();
            rotation.yaw = calculate_relative_teleport(
                TeleportFlag::RelYaw,
                flags,
                rotation.yaw,
                -teleport.yaw as f64 * (PI / 180.0),
            );

            rotation.pitch = -((calculate_relative_teleport(
                TeleportFlag::RelPitch,
                flags,
                (-rotation.pitch) * (180.0 / PI) + 180.0,
                teleport.pitch as f64,
            ) - 180.0)
                * (PI / 180.0));
            }

            let mut velocity = entities
                .world.entity_mut(player)
                .get_mut::<crate::entity::Velocity>()
                .unwrap();

            if (flags & (TeleportFlag::RelX as u8)) == 0 {
                velocity.velocity.x = 0.0;
            }
            if (flags & (TeleportFlag::RelY as u8)) == 0 {
                velocity.velocity.y = 0.0;
            }
            if (flags & (TeleportFlag::RelZ as u8)) == 0 {
                velocity.velocity.z = 0.0;
            }

            if let Some(teleport_id) = teleport.teleport_id {
                self.write_packet(packet::play::serverbound::TeleportConfirm {
                    teleport_id: protocol::VarInt(teleport_id),
                });
            }
        }
    }

    fn on_block_entity_update(
        &self,
        block_update: mapped_packet::play::clientbound::UpdateBlockEntity,
    ) {
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

    /*fn on_block_entity_update_data(
        &self,
        _block_update: packet::play::clientbound::UpdateBlockEntity_Data,
    ) {
        // TODO: handle UpdateBlockEntity_Data for 1.7, decompress gzipped_nbt
    }*/

    fn on_sign_update(&self, mut update_sign: mapped_packet::play::clientbound::UpdateSign) {
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

    fn on_player_info_string(
        &self,
        _player_info: mapped_packet::play::clientbound::PlayerInfo_String,
    ) {
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

    fn on_player_info(&self, player_info: mapped_packet::play::clientbound::PlayerInfo) {
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
                        let entities = self.entities.clone();
                        let mut entities = entities.write();
                        let mut model = entities
                            .world.entity_mut(self.player.clone().write().unwrap())
                            .get_mut::<entity::player::PlayerModel>()
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

    fn on_servermessage(&self, message: mapped_packet::play::clientbound::ServerMessage) {
        debug!("Received chat message: {}", message.message);
        self.hud_context
            .clone()
            .write()
            .display_message_in_chat(message.message);
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
                self.on_block_entity_update(mapped_packet::play::clientbound::UpdateBlockEntity {
                    location: Position::new(x, y, z),
                    action,
                    nbt: Some(block_entity.clone()),
                    data_length: None,
                    gzipped_nbt: None,
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
        chunk_data: mapped_packet::play::clientbound::ChunkData_Biomes3D_i32,
    ) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_biomes3d_bool(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_Biomes3D_bool,
    ) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_biomes3d(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_Biomes3D,
    ) {
        self.world
            .clone()
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data(&self, chunk_data: mapped_packet::play::clientbound::ChunkData) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_heightmap(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_HeightMap,
    ) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_no_entities(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_NoEntities,
    ) {
        self.world
            .clone()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
    }

    fn on_chunk_data_no_entities_u16(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_NoEntities_u16,
    ) {
        let chunk_meta = vec![crate::protocol::packet::ChunkMeta {
            x: chunk_data.chunk_x,
            z: chunk_data.chunk_z,
            bitmask: chunk_data.bitmask,
        }];
        let skylight = false;
        self.world
            .clone()
            .load_chunks18(chunk_data.new, skylight, &chunk_meta, chunk_data.data)
            .unwrap();
    }

    fn on_chunk_data_17(&self, chunk_data: mapped_packet::play::clientbound::ChunkData_17) {
        self.world
            .clone()
            .load_chunk17(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask,
                chunk_data.add_bitmask,
                chunk_data.compressed_data,
            )
            .unwrap();
    }

    fn on_chunk_data_bulk(&self, bulk: mapped_packet::play::clientbound::ChunkDataBulk) {
        let new = true;
        self.world
            .clone()
            .load_chunks18(
                new,
                bulk.skylight,
                &bulk.chunk_meta,
                bulk.chunk_data.to_vec(),
            )
            .unwrap();
    }

    fn on_chunk_data_bulk_17(&self, bulk: mapped_packet::play::clientbound::ChunkDataBulk_17) {
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

    fn on_chunk_unload(&self, chunk_unload: mapped_packet::play::clientbound::ChunkUnload) {
        self.world.clone().unload_chunk(
            chunk_unload.x,
            chunk_unload.z,
            &mut self.entities.clone().write(),
        );
    }

    fn on_block_change_in_world(&self, location: Position, id: i32) {
        let world = self.world.clone();
        let modded_block_ids = world.modded_block_ids.clone();
        let block = world.id_map.by_vanilla_id(id as usize, modded_block_ids);
        world.set_block(location, block)
    }

    fn on_block_change(&self, block_change: mapped_packet::play::clientbound::BlockChange) {
        self.on_block_change_in_world(block_change.location, block_change.block_id)
    }

    fn on_multi_block_change(
        &self,
        block_change: mapped_packet::play::clientbound::MultiBlockChange,
    ) {
        let ox = block_change.chunk_x << 4;
        let oz = block_change.chunk_z << 4;
        for record in block_change.records {
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
            self.on_block_change_in_world(
                Position::new(
                    ox + (record.xz >> 4) as i32,
                    record.y as i32,
                    oz + (record.xz & 0xF) as i32,
                ),
                record.block_id as i32,
            );
        }
    }

    pub fn on_update_health(&self, health: f32, food: u8, saturation: u8) {
        self.hud_context
            .clone()
            .write()
            .update_health_and_food(health, food, saturation);
        if health <= 0.0 && !*self.dead.read() {
            *self.dead.write() = true;
            self.screen_sys.close_closable_screens();
            self.screen_sys
                .clone()
                .add_screen(Box::new(Respawn::new(0))); // TODO: Use the correct score!
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
