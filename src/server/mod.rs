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

use crate::ecs::{Manager, SystemExecStage};
use crate::entity;
use crate::entity::player::{create_local, PlayerModel, PlayerMovement};
use crate::entity::{EntityType, GameInfo, Gravity, MouseButtons, TargetPosition, TargetRotation};
use crate::format;
use crate::inventory::material::versions::to_material;
use crate::inventory::{inventory_from_type, InventoryContext, InventoryType, Item};
use crate::particle::block_break_effect::{BlockBreakEffect, BlockEffectData};
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
use crate::world::{self, World};
use crate::world::{CPos, LightData, LightUpdate};
use crate::{ecs, Game};
use arc_swap::ArcSwapOption;
use atomic_float::AtomicF64;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use bevy_ecs::prelude::Entity;
use bevy_ecs::schedule::{IntoSystemConfigs, Schedule};
use bevy_ecs::system::{Commands, Res, ResMut, Resource};
use cgmath::prelude::*;
use cgmath::Vector3;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;
use instant::{Duration, Instant};
use leafish_protocol::format::Component;
use leafish_protocol::item::Stack;
use leafish_protocol::protocol::login::Account;
use leafish_protocol::protocol::mapped_packet::MappablePacket;
use leafish_protocol::protocol::mapped_packet::MappedPacket;
use leafish_protocol::protocol::packet::Hand;
use leafish_protocol::protocol::Conn;
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use parking_lot::RwLock;
use rand::Rng;
use rayon::ThreadPoolBuilder;
use shared::Version;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use self::sun::SunModel;

pub mod plugin_messages;
mod sun;
pub mod target;

#[derive(Default)]
pub struct DisconnectData {
    pub disconnect_reason: Option<format::Component>,
    just_disconnected: bool,
}

#[derive(Resource)]
struct WorldData {
    // FIXME: move to world?
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
    pub(crate) disconnect_gracefully: AtomicBool,
    pub protocol_version: i32,
    pub mapped_protocol_version: Version,
    forge_mods: Vec<forge::ForgeMod>,
    pub disconnect_data: Arc<RwLock<DisconnectData>>,

    pub world: Arc<world::World>,
    pub entities: Arc<RwLock<ecs::Manager>>,

    resources: Arc<RwLock<resources::Manager>>,
    version: AtomicUsize,

    // Entity accessors
    pub player: ArcSwapOption<(i32, Entity)>,
    entity_map: Arc<RwLock<HashMap<i32, Entity, BuildHasherDefault<FNVHash>>>>,
    players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,

    tick_timer: AtomicF64,
    entity_tick_timer: AtomicF64,
    pub received_chat_at: ArcSwapOption<Instant>,

    target_info: Arc<RwLock<target::Info>>,
    pub render_list_computer: Sender<bool>,
    pub render_list_computer_notify: Receiver<bool>,
    pub hud_context: Arc<RwLock<HudContext>>,
    pub inventory_context: Arc<RwLock<InventoryContext>>,
    pub dead: AtomicBool,
    just_died: AtomicBool,
    last_chat_open: AtomicBool,
    pub chat_open: AtomicBool,
    pub chat_ctx: Arc<ChatContext>,
    screen_sys: Arc<ScreenSystem>,
    renderer: Arc<Renderer>,
    active_block_break_anims: Arc<DashMap<i32, Entity>>,
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
        renderer: Arc<Renderer>,
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
        renderer: Arc<Renderer>,
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
        server.hud_context.write().server = Some(server.clone());

        inner_server.replace(server.clone());
        render_list_computer.0.send(true).unwrap();
        server
    }

    #[allow(unused_must_use)]
    fn spawn_reader(mut read: protocol::Conn, server: Arc<Mutex<Option<Arc<Server>>>>) {
        thread::spawn(move || {
            let threads = ThreadPoolBuilder::new().num_threads(8).build().unwrap();
            loop {
                let server = server.lock().as_ref().unwrap().clone();
                let pck = read.read_packet();
                match pck {
                    Ok(pck) => match pck.map() {
                        MappedPacket::KeepAliveClientbound(keep_alive) => {
                            packet::send_keep_alive(
                                server.conn.write().as_mut().unwrap(),
                                keep_alive.id,
                            )
                            .map_err(|_| server.disconnect_closed(None));
                        }
                        MappedPacket::ChunkData_NoEntities(chunk_data) => {
                            let sky_light = server.world.dimension.load().has_sky_light();
                            server.on_chunk_data_no_entities(chunk_data, sky_light);
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
                            let sky_light = server.world.dimension.load().has_sky_light();
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d(chunk_data, sky_light);
                            });
                        }
                        MappedPacket::ChunkData_Biomes3D_i32(chunk_data) => {
                            let sky_light = server.world.dimension.load().has_sky_light();
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d_varint(chunk_data, sky_light);
                            });
                        }
                        MappedPacket::ChunkData_Biomes3D_bool(chunk_data) => {
                            let sky_light = server.world.dimension.load().has_sky_light();
                            threads.spawn(move || {
                                server.on_chunk_data_biomes3d_bool(chunk_data, sky_light);
                            });
                        }
                        MappedPacket::ChunkData(chunk_data) => {
                            let sky_light = server.world.dimension.load().has_sky_light();
                            threads.spawn(move || {
                                server.on_chunk_data(chunk_data, sky_light);
                            });
                        }
                        MappedPacket::ChunkData_HeightMap(chunk_data) => {
                            let sky_light = server.world.dimension.load().has_sky_light();
                            threads.spawn(move || {
                                server.on_chunk_data_heightmap(chunk_data, sky_light);
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
                            if let Some(entity) = server.entity_map.read().get(&look.entity_id) {
                                let mut entities = server.entities.write();
                                let mut entity = entities.world.get_entity_mut(*entity).unwrap();
                                let mut rotation = entity.get_mut::<TargetRotation>().unwrap();
                                rotation.yaw = -(look.head_yaw as f64 / 256.0) * PI * 2.0;
                            }
                        }
                        MappedPacket::JoinGame(join) => {
                            let protocol::mapped_packet::play::clientbound::JoinGame {
                                gamemode,
                                entity_id,
                                dimension_id,
                                dimension_name,
                                dimension,
                                world_name,
                                ..
                            } = join;

                            server.on_game_join(gamemode, entity_id);

                            let dimension = dimension_id
                                .map(world::Dimension::from_index)
                                .or_else(|| dimension_name.map(|d| world::Dimension::from_name(&d)))
                                .or_else(|| world_name.map(|d| world::Dimension::from_name(&d)))
                                .or_else(|| dimension.map(|d| world::Dimension::from_tag(&d)));

                            if let Some(dimension) = dimension {
                                server.world.set_dimension(dimension);
                            }
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
                            server.on_confirm_transaction(
                                transaction.id,
                                transaction.action_number,
                                transaction.accepted,
                            );

                            read.write_packet(
                                packet::play::serverbound::ConfirmTransactionServerbound {
                                    id: transaction.id,
                                    action_number: transaction.action_number,
                                    accepted: transaction.accepted,
                                },
                            )
                            .unwrap();
                        }
                        MappedPacket::UpdateLight(update_light) => {
                            server.world.lighting_cache.write().insert(
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
                                .write()
                                .update_exp(set_exp.experience_bar, set_exp.level);
                        }
                        MappedPacket::SetCurrentHotbarSlot(set_slot) => {
                            if set_slot.slot <= 8 {
                                server.inventory_context.write().hotbar_index = set_slot.slot;
                                server.hud_context.write().update_slot_index(set_slot.slot);
                            } else {
                                warn!("The server tried to set the hotbar slot to {}, although it has to be in a range of 0-8! Did it try to crash you?", set_slot.slot);
                            }
                        }
                        MappedPacket::WindowItems(items) => {
                            for item in items.items.into_iter().enumerate() {
                                server.on_set_slot(items.id as i16, item.0 as i16, item.1);
                            }
                        }
                        MappedPacket::WindowProperty(data) => {
                            let win_id: i32 = data.id as i32;
                            if let Some(inv) =
                                server.inventory_context.read().safe_inventory.clone()
                            {
                                if inv.read().id() == win_id {
                                    inv.write()
                                        .handle_property_packet(data.property, data.value);
                                } else {
                                    warn!("The server has send information about a inventory that is not open. Did it try to crash you?");
                                }
                            }
                        }
                        MappedPacket::WindowSetSlot(set_slot) => {
                            server.on_set_slot(set_slot.id as i16, set_slot.slot, set_slot.item);
                        }
                        MappedPacket::WindowClose(_close) => {
                            server
                                .inventory_context
                                .write()
                                .try_close_inventory(&server.screen_sys);
                        }
                        MappedPacket::WindowOpen(open) => {
                            let inv_type = if let Some(name) = &open.ty_name {
                                InventoryType::from_name(name, open.slot_count.unwrap())
                            } else {
                                let version = server.mapped_protocol_version;
                                InventoryType::from_id(version, open.ty.unwrap())
                            };

                            if let Some(inv_type) = inv_type {
                                let inventory = inventory_from_type(
                                    inv_type,
                                    open.title,
                                    &server.renderer,
                                    server.inventory_context.read().base_slots.clone(),
                                    open.id,
                                );
                                if let Some(inventory) = inventory {
                                    server.inventory_context.write().open_inventory(
                                        inventory.clone(),
                                        &server.screen_sys,
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
                            if block_break.stage >= 10 || block_break.stage < 1 {
                                if let Some(break_impl) = server
                                    .active_block_break_anims
                                    .remove(&block_break.entity_id)
                                {
                                    server.entities.write().world.despawn(break_impl.1);
                                }
                            } else if let Some(anim_ent) = server
                                .active_block_break_anims
                                .clone()
                                .get(&block_break.entity_id)
                            {
                                let mut entities = server.entities.write();
                                let mut anim =
                                    entities.world.get_entity_mut(*anim_ent.value()).unwrap();
                                let effect = anim.get_mut::<BlockBreakEffect>();
                                if let Some(mut effect) = effect {
                                    effect.update(block_break.stage);
                                }
                            } else {
                                let mut entities = server.entities.write();
                                let mut entity = entities.world.spawn_empty();
                                entity.insert(BlockEffectData {
                                    position: Vector3::new(
                                        block_break.location.x as f64,
                                        block_break.location.y as f64,
                                        block_break.location.z as f64,
                                    ),
                                    status: block_break.stage,
                                });
                                entity.insert(crate::particle::ParticleType::BlockBreak);
                                let entity = entity.id();
                                server
                                    .active_block_break_anims
                                    .insert(block_break.entity_id, entity);
                            }
                        }
                        _ => {
                            // debug!("other packet!");
                        }
                    },
                    Err(err) => {
                        if server.disconnect_data.read().disconnect_reason.is_none() {
                            server.disconnect_data.write().disconnect_reason.replace(
                                Component::new(format::ComponentType::new(
                                    &format!("An error occurred while reading a packet: {}", err),
                                    None,
                                )),
                            );
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
        renderer: Arc<Renderer>,
    ) -> (Sender<bool>, Receiver<bool>) {
        let (tx, rx) = unbounded();
        let (etx, erx) = unbounded();
        thread::spawn(move || loop {
            let _ = rx.recv().unwrap();
            let server = server.lock();
            server
                .as_ref()
                .unwrap()
                .world
                .compute_render_list(renderer.clone());
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
        renderer: Arc<Renderer>,
    ) -> Self {
        let world = Arc::new(world::World::new(protocol_version, light_updater));
        let mapped_protocol_version = Version::from_id(protocol_version as u32);
        let inventory_context = Arc::new(RwLock::new(InventoryContext::new(
            mapped_protocol_version,
            &renderer,
            hud_context.clone(),
            conn.clone(),
        )));
        let mut entities = Manager::default();
        // FIXME: fix threading modes (make some systems execute in parallel and others in sync)
        entities
            .world
            .insert_resource(RendererResource(renderer.clone()));
        entities
            .world
            .insert_resource(ScreenSystemResource(screen_sys.clone()));
        entities.world.insert_resource(ConnResource(conn.clone()));
        entities.world.insert_resource(entity::GameInfo::new());
        entities.world.insert_resource(WorldResource(world.clone()));
        entities
            .world
            .insert_resource(InventoryContextResource(inventory_context.clone()));
        entities.world.insert_resource(DeltaResource(0.0));
        entities.world.insert_resource(WorldData::default());
        entities.world.insert_resource(RenderCtxResource::default());
        entity::add_systems(&mut entities.schedule.write());
        add_systems(&mut entities.schedule.write());

        hud_context.write().slots = Some(inventory_context.read().base_slots.clone());

        let version = resources.read().version();
        Self {
            uuid,
            conn,
            disconnect_gracefully: Default::default(),
            protocol_version,
            mapped_protocol_version,
            forge_mods,
            disconnect_data: Arc::new(RwLock::new(DisconnectData::default())),

            world,
            version: AtomicUsize::new(version),
            resources,

            entities: Arc::new(RwLock::new(entities)),
            player: ArcSwapOption::new(None),
            entity_map: Arc::new(RwLock::new(HashMap::with_hasher(
                BuildHasherDefault::default(),
            ))),
            players: Arc::new(RwLock::new(HashMap::with_hasher(
                BuildHasherDefault::default(),
            ))),

            tick_timer: AtomicF64::new(0.0),
            entity_tick_timer: AtomicF64::new(0.0),
            received_chat_at: ArcSwapOption::new(None),

            target_info: Arc::new(RwLock::new(target::Info::new())),
            render_list_computer,
            render_list_computer_notify,
            hud_context,
            inventory_context,
            dead: AtomicBool::new(false),
            just_died: AtomicBool::new(false),
            last_chat_open: AtomicBool::new(false),
            chat_open: AtomicBool::new(false),
            chat_ctx: Arc::new(ChatContext::new()),
            screen_sys,
            renderer,
            active_block_break_anims: Arc::new(Default::default()),
        }
    }

    pub fn disconnect(&self, reason: Option<format::Component>) {
        self.conn.write().take().unwrap().close();
        self.disconnect_data.write().disconnect_reason = reason;
        if let Some(player) = self.player.swap(None) {
            // TODO: Is this even required if we have the despawn code below?
            self.entities.write().world.despawn(player.1);
        }
        for entity in &*self.entity_map.write() {
            if self.entities.read().world.get_entity(*entity.1).is_some() {
                self.entities.write().world.despawn(*entity.1);
            }
        }
        self.disconnect_data.write().just_disconnected = true;
    }

    pub fn disconnect_closed(&self, reason: Option<format::Component>) {
        self.disconnect_data.write().disconnect_reason = reason;
        self.disconnect_data.write().just_disconnected = true;
        self.disconnect_gracefully.store(true, Ordering::Relaxed);
    }

    pub fn finish_disconnect(&self) {
        self.conn.write().take().unwrap().close();
        if let Some(player) = self.player.swap(None) {
            // TODO: Is this even required if we have the despawn code below?
            self.entities.write().world.despawn(player.1);
        }
        for entity in &*self.entity_map.write() {
            if self.entities.read().world.get_entity(*entity.1).is_some() {
                self.entities.write().world.despawn(*entity.1);
            }
        }
        self.entities
            .write()
            .world
            .remove_resource::<SunModelResource>();
        // FIXME: remove other resources!
    }

    pub fn is_connected(&self) -> bool {
        self.conn.read().is_some()
    }

    pub fn tick(&self, delta: f64, game: &mut Game) {
        {
            let mut entities = self.entities.write();
            // FIXME: is there another way to do this?
            entities.world.resource_mut::<DeltaResource>().0 = delta;
            let start = SystemTime::now();
            let time = start.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64; // FIXME: use safer conversion
            let fps_res = entities.world.get_resource::<RenderCtxResource>().unwrap();
            if fps_res.0.frame_start.load(Ordering::Acquire) + 1000 < time {
                self.hud_context
                    .write()
                    .update_fps(fps_res.0.fps.load(Ordering::Acquire));
                fps_res.0.frame_start.store(time, Ordering::Release);
                fps_res.0.fps.store(0, Ordering::Release);
            } else {
                fps_res
                    .0
                    .fps
                    .store(fps_res.0.fps.load(Ordering::Acquire) + 1, Ordering::Release);
            }
        }
        let renderer = self.renderer.clone();
        let chat_open = self.chat_open.load(Ordering::Acquire);
        if chat_open != self.last_chat_open.load(Ordering::Acquire) {
            self.last_chat_open.store(chat_open, Ordering::Release);
            if chat_open {
                game.screen_sys
                    .add_screen(Box::new(Chat::new(self.chat_ctx.clone())));
            } else {
                game.screen_sys.pop_screen();
            }
        }
        let version = self.resources.read().version();
        if version != self.version.load(Ordering::Acquire) {
            self.version.store(version, Ordering::Release);
            self.world.flag_dirty_all();
        }
        {
            {
                let mut entities = self.entities.write();
                if !entities.world.contains_resource::<SunModelResource>() {
                    // TODO: Check if the world type actually needs a sun
                    entities
                        .world
                        .insert_resource(SunModelResource(SunModel::new(renderer.clone())));
                }
            }
            // Copy to camera
            if let Some(player) = self.player.load().as_ref() {
                let entities = self.entities.read();
                let position = entities
                    .world
                    .get_entity(player.1)
                    .unwrap()
                    .get::<crate::entity::Position>()
                    .unwrap();
                let rotation = entities
                    .world
                    .get_entity(player.1)
                    .unwrap()
                    .get::<crate::entity::Rotation>()
                    .unwrap();
                renderer.camera.lock().pos = cgmath::Point3::from_vec(
                    position.position + cgmath::Vector3::new(0.0, 1.62, 0.0),
                );
                renderer.camera.lock().yaw = rotation.yaw;
                renderer.camera.lock().pitch = rotation.pitch;
            }
        }
        self.entity_tick(delta, game.focused, self.dead.load(Ordering::Acquire));

        self.tick_timer.store(
            self.tick_timer.load(Ordering::Acquire) + delta,
            Ordering::Release,
        );
        while self.tick_timer.load(Ordering::Acquire) >= 3.0 && self.is_connected() {
            self.minecraft_tick(game);
            self.tick_timer.store(
                self.tick_timer.load(Ordering::Acquire) - 3.0,
                Ordering::Release,
            );
        }

        self.update_time(&renderer);
        // FIXME: tick sun in between!
        // self.world.tick(&mut self.entities.write());

        if self.player.load().as_ref().is_some() {
            if self.just_died.load(Ordering::Acquire) {
                self.just_died.store(false, Ordering::Release);
                game.screen_sys.close_closable_screens();
                game.screen_sys.add_screen(Box::new(Respawn::new(0))); // TODO: Use the correct score!
            }
            if let Some((pos, bl, _, _)) = target::trace_ray(
                &self.world,
                4.0,
                renderer.camera.lock().pos.to_vec(),
                renderer.view_vector.lock().cast().unwrap(),
                target::test_block,
            ) {
                self.target_info.write().update(renderer.clone(), pos, bl);
            } else {
                self.target_info.write().clear();
            }
        } else {
            self.target_info.write().clear();
        }
    }

    fn entity_tick(&self, delta: f64, _focused: bool, _dead: bool) {
        let mut entities = self.entities.write();
        {
            let mut game_info = entities.world.get_resource_mut::<GameInfo>().unwrap();
            // Update the game's state for entities to read
            game_info.delta = delta;
        }

        if self.is_connected() || self.disconnect_data.read().just_disconnected {
            // Allow an extra tick when disconnected to clean up
            self.disconnect_data.write().just_disconnected = false;
            self.entity_tick_timer.store(
                self.entity_tick_timer.load(Ordering::Acquire) + delta,
                Ordering::Release,
            );
            entities.world.clear_trackers();
            let entity_schedule = entities.entity_schedule.clone();
            while self.entity_tick_timer.load(Ordering::Acquire) >= 3.0 {
                entity_schedule.write().run(&mut entities.world);
                self.entity_tick_timer.store(
                    self.entity_tick_timer.load(Ordering::Acquire) - 3.0,
                    Ordering::Release,
                );
            }
            let schedule = entities.schedule.clone();
            schedule.write().run(&mut entities.world);
        }
    }

    /*
    pub fn remove(&mut self, renderer: &mut render::Renderer) {
        let world = self.world.clone();
        let entities = self.entities.read();
        let cleanup_manager = entities.world.get_resource::<CleanupManager>().unwrap();
        cleanup_manager.cleanup_all();
        if let Some(sun_model) = self.sun_model.write().as_mut() {
            sun_model.remove(renderer);
        }
        self.target_info.clone().write().clear(renderer);
    }*/

    fn update_time(&self, renderer: &Arc<render::Renderer>) {
        renderer.light_data.lock().sky_offset = self.calculate_sky_offset();
    }

    fn calculate_sky_offset(&self) -> f32 {
        use std::f32::consts::PI;
        let mut offset = ((1.0
            + self
                .entities
                .read()
                .world
                .resource::<WorldData>()
                .world_time as f32)
            / 24000.0)
            - 0.25;
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

    #[allow(unused_must_use)]
    pub fn minecraft_tick(&self, game: &mut Game) {
        if let Some(player) = self.player.load().as_ref() {
            let mut entities = self.entities.write();
            let on_ground = {
                let mut player = entities.world.entity_mut(player.1);
                let mut movement = player.get_mut::<PlayerMovement>().unwrap();
                // Force the server to know when touched the ground
                // otherwise if it happens between ticks the server
                // will think we are flying.
                if movement.did_touch_ground {
                    movement.did_touch_ground = false;
                    Some(true)
                } else {
                    None
                }
            }
            .unwrap_or_else(|| {
                entities
                    .world
                    .entity(player.1)
                    .get::<Gravity>()
                    .map_or(false, |v| v.on_ground)
            });

            let position = entities
                .world
                .entity(player.1)
                .get::<TargetPosition>()
                .unwrap();
            let rotation = entities
                .world
                .entity(player.1)
                .get::<crate::entity::Rotation>()
                .unwrap();

            // Sync our position to the server
            // Use the smaller packets when possible
            packet::send_position_look(
                self.conn.write().as_mut().unwrap(),
                &position.position,
                rotation.yaw as f32,
                rotation.pitch as f32,
                on_ground,
            )
            .map_err(|_| self.disconnect_closed(None));

            if !game.focused {
                let mut player = entities.world.entity_mut(player.1);
                let mut mouse_buttons = player.get_mut::<MouseButtons>().unwrap();
                mouse_buttons.left = false;
                mouse_buttons.right = false;
            }
        }
    }

    pub fn key_press(&self, down: bool, key: Actionkey, focused: &mut bool) -> bool {
        if *focused || key == Actionkey::OpenInv || key == Actionkey::ToggleChat {
            let mut state_changed = false;
            if let Some(player) = self.player.load().as_ref() {
                if let Some(mut movement) = self
                    .entities
                    .write()
                    .world
                    .entity_mut(player.1)
                    .get_mut::<PlayerMovement>()
                {
                    state_changed = movement.pressed_keys.get(&key).map_or(false, |v| *v) != down;
                    movement.pressed_keys.insert(key, down);
                }
            }
            match key {
                Actionkey::OpenInv => {
                    if down {
                        let player_inv = self.inventory_context.read().player_inventory.clone();
                        self.inventory_context.write().open_inventory(
                            player_inv,
                            &self.screen_sys,
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

    pub fn on_left_click(&self, focused: bool) {
        if focused {
            let mut entities = self.entities.write();
            // check if the player exists, as it might not be initialized very early on server join
            if let Some(player) = self.player.load().as_ref() {
                let mut player = entities.world.entity_mut(player.1);
                let mut mouse_buttons = player.get_mut::<MouseButtons>().unwrap();
                mouse_buttons.left = true;
            }
        } else {
            self.inventory_context.write().on_click()
        }
    }

    pub fn on_release_left_click(&self, focused: bool) {
        if focused {
            let mut entities = self.entities.write();
            // check if the player exists, as it might not be initialized very early on server join
            if let Some(player) = self.player.load().as_ref() {
                let mut player = entities.world.entity_mut(player.1);
                let mut mouse_buttons = player.get_mut::<MouseButtons>().unwrap();
                mouse_buttons.left = false;
            }
        }
        // TODO: Pass events into inventory context when not focused
    }

    #[allow(unused_must_use)]
    pub fn on_right_click(&self, focused: bool) {
        if self.player.load().as_ref().is_some() && focused {
            let gamemode = *self
                .entities
                .read()
                .world
                .entity(self.player.load().as_ref().unwrap().1)
                .get::<GameMode>()
                .unwrap();
            if gamemode.can_interact_with_world() {
                // TODO: Check this
                if let Some((pos, _, face, at)) = target::trace_ray(
                    &self.world,
                    4.0,
                    self.renderer.camera.lock().pos.to_vec(),
                    self.renderer.view_vector.lock().cast().unwrap(),
                    target::test_block,
                ) {
                    let hud_context = self.hud_context.clone();
                    packet::send_block_place(
                        self.conn.write().as_mut().unwrap(),
                        pos,
                        face.index() as u8,
                        at,
                        Hand::MainHand,
                        Box::new(move || {
                            hud_context
                                .read()
                                .slots
                                .as_ref()
                                .unwrap()
                                .clone()
                                .read()
                                .get_item((27 + hud_context.read().get_slot_index()) as u16)
                                .as_ref()
                                .map(|item| item.stack.clone())
                        }),
                    )
                    .map_err(|_| self.disconnect_closed(None));
                    packet::send_arm_swing(
                        self.conn.clone().write().as_mut().unwrap(),
                        Hand::MainHand,
                    )
                    .map_err(|_| self.disconnect_closed(None));
                }
            }

            let mut entities = self.entities.write();
            // check if the player exists, as it might not be initialized very early on server join
            if let Some(player) = self.player.load().as_ref() {
                let mut player = entities.world.entity_mut(player.1);
                let mut mouse_buttons = player.get_mut::<MouseButtons>().unwrap();
                mouse_buttons.right = true;
            }
        }
        // TODO: Pass events into inventory context when not focused
    }

    pub fn on_release_right_click(&self, focused: bool) {
        if focused {
            let mut entities = self.entities.write();
            // check if the player exists, as it might not be initialized very early on server join
            if let Some(player) = self.player.load().as_ref() {
                let mut player = entities.world.entity_mut(player.1);
                let mut mouse_buttons = player.get_mut::<MouseButtons>().unwrap();
                mouse_buttons.right = false;
            }
        }
        // TODO: Pass events into inventory context when not focused
    }

    pub fn on_cursor_moved(&self, x: f64, y: f64) {
        let mut inventory = self.inventory_context.write();
        inventory.on_cursor_moved(x, y);
    }

    pub fn write_packet<T: protocol::PacketType>(&self, p: T) {
        let mut conn = self.conn.write();
        if conn.is_some() {
            let result = conn.as_mut().unwrap().write_packet(p);
            if result.is_ok() {
                return;
            }
        }
        self.disconnect(Some(Component::new(format::ComponentType::Text {
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
                        let mut mod_ids = self.world.modded_block_ids.load().as_ref().clone();
                        for m in mappings.data {
                            let (namespace, name) = m.name.split_at(1);
                            if namespace == protocol::forge::BLOCK_NAMESPACE {
                                mod_ids.insert(m.id.0 as usize, name.to_string());
                            }
                        }
                        self.world.modded_block_ids.store(Arc::new(mod_ids));
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
                            let mut mod_ids = self.world.modded_block_ids.load().as_ref().clone();
                            for m in ids.data {
                                mod_ids.insert(m.id.0 as usize, m.name);
                            }
                            self.world.modded_block_ids.store(Arc::new(mod_ids));
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
            .write()
            .as_mut()
            .unwrap()
            .write_fmlhs_plugin_message(msg); // TODO handle errors
    }

    fn write_plugin_message(&self, channel: &str, data: &[u8]) {
        let _ = self
            .conn
            .write()
            .as_mut()
            .unwrap()
            .write_plugin_message(channel, data); // TODO handle errors
    }

    fn on_set_slot(&self, inventory_id: i16, slot: i16, item: Option<Stack>) {
        /*println!(
            "set item {:?} to slot {} to inv {}",
            item.as_ref(),
            slot,
            inventory_id
        );*/
        let top_inventory = &self.inventory_context;
        let inventory = if inventory_id == -1 || inventory_id == 0 {
            top_inventory.read().player_inventory.clone() // TODO: This caused a race condition, check why!
        } else if let Some(inventory) = top_inventory.read().safe_inventory.as_ref() {
            inventory.clone()
        } else {
            println!("Couldn't set item to slot {}", slot);
            return;
        };
        let curr_slots = inventory.read().size();
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
            inventory.write().set_item(slot as u16, item);
            self.hud_context
                .write()
                .dirty_slots
                .store(true, Ordering::Relaxed);
        }
    }

    fn on_confirm_transaction(&self, id: u8, action_number: i16, accepted: bool) {
        self.inventory_context
            .write()
            .on_confirm_transaction(id, action_number, accepted);
    }

    #[allow(unused_must_use)]
    fn on_game_join(&self, gamemode: u8, entity_id: i32) {
        let gamemode = GameMode::from_int((gamemode & 0x7) as i32);
        let player = entity::player::create_local(&mut self.entities.clone().write());
        if let Some(info) = self.players.read().get(&self.uuid) {
            let mut entities = self.entities.write();
            let mut player = entities.world.entity_mut(player);
            let mut model = player.get_mut::<PlayerModel>().unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.hud_context.clone().write().update_game_mode(gamemode);
        *self
            .entities
            .write()
            .world
            .entity_mut(player)
            .get_mut::<GameMode>()
            .unwrap() = gamemode;
        self.entities
            .write()
            .world
            .entity_mut(player)
            .get_mut::<PlayerMovement>()
            .unwrap()
            .flying = gamemode.can_fly();

        self.entity_map.write().insert(entity_id, player);
        self.player.store(Some(Arc::new((entity_id, player))));

        // Let the server know who we are
        let brand = plugin_messages::Brand {
            brand: "leafish".into(),
        };
        brand.write_to(self.conn.write().as_mut().unwrap());

        packet::send_client_settings(
            self.conn.write().as_mut().unwrap(),
            "en_us".to_string(),
            8,
            0,
            true,
            127,
            Hand::MainHand,
        )
        .map_err(|_| self.disconnect_closed(None)); // TODO: Make these configurable!
    }

    fn on_respawn(&self, respawn: mapped_packet::play::clientbound::Respawn) {
        let protocol::mapped_packet::play::clientbound::Respawn {
            gamemode,
            dimension,
            dimension_name,
            dimension_tag,
            world_name,
            ..
        } = respawn;

        for entity in &*self.entity_map.write() {
            if self.entities.read().world.get_entity(*entity.1).is_some() {
                self.entities.write().world.despawn(*entity.1);
            }
        }

        let entity_id = self.player.load().as_ref().unwrap().0;
        let local_player = create_local(&mut self.entities.write());
        self.player.store(Some(Arc::new((entity_id, local_player))));
        let gamemode = GameMode::from_int((gamemode & 0x7) as i32);

        if let Some(player) = self.player.load().as_ref() {
            self.hud_context.write().update_game_mode(gamemode);

            *self
                .entities
                .write()
                .world
                .entity_mut(player.1)
                .get_mut::<GameMode>()
                .unwrap() = gamemode;
            self.entities
                .write()
                .world
                .entity_mut(player.1)
                .get_mut::<PlayerMovement>()
                .unwrap()
                .flying = gamemode.can_fly();
        }
        if self.dead.load(Ordering::Acquire) {
            self.dead.store(false, Ordering::Release);
            self.just_died.store(false, Ordering::Release);
            let mut hud_context = self.hud_context.write();
            hud_context.update_health_and_food(20.0, 20, 0); // TODO: Verify this!
            hud_context.update_slot_index(0);
            hud_context.update_exp(0.0, 0);
            hud_context.update_absorbtion(0.0);
            hud_context.update_armor(0);
            // hud_context.update_breath(-1); // TODO: Fix this!
            drop(hud_context);
            self.screen_sys.pop_screen();
        }
        self.entity_map.write().insert(entity_id, local_player);

        let dimension = dimension
            .map(world::Dimension::from_index)
            .or_else(|| dimension_name.map(|d| world::Dimension::from_name(&d)))
            .or_else(|| world_name.map(|d| world::Dimension::from_name(&d)))
            .or_else(|| dimension_tag.map(|d| world::Dimension::from_tag(&d)));

        if let Some(dimension) = dimension {
            self.world.set_dimension(dimension);
        }
    }

    // TODO: make use of "on_disconnect"
    #[allow(dead_code)]
    fn on_disconnect(&self, disconnect: packet::play::clientbound::Disconnect) {
        self.disconnect(Some(disconnect.reason));
    }

    fn on_time_update(&self, time_update: mapped_packet::play::clientbound::TimeUpdate) {
        // FIXME: use events that are passed to the world instead of locking up!
        let mut entities = self.entities.write();
        let mut world_data = entities.world.resource_mut::<WorldData>();
        world_data.world_age = time_update.time_of_day;
        world_data.world_time_target = (time_update.time_of_day % 24000) as f64;
        if world_data.world_time_target < 0.0 {
            world_data.world_time_target *= -1.0;
            world_data.tick_time = false;
        } else {
            world_data.tick_time = true;
        }
    }

    fn on_game_state_change(&self, game_state: mapped_packet::play::clientbound::ChangeGameState) {
        if game_state.reason == 3 {
            if let Some(player) = self.player.load().as_ref() {
                let gamemode = GameMode::from_int(game_state.value as i32);
                self.hud_context.clone().write().update_game_mode(gamemode);
                *self
                    .entities
                    .write()
                    .world
                    .entity_mut(player.1)
                    .get_mut::<GameMode>()
                    .unwrap() = gamemode;
                self.entities
                    .write()
                    .world
                    .entity_mut(player.1)
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
            let entity = entity_type.create_entity(&mut self.entities.write(), x, y, z, yaw, pitch);
            if let Some(entity) = entity {
                self.entity_map.write().insert(entity_id, entity);
                println!("spawned {} {:?}", ty, entity_type);
            }
        }
    }

    fn on_entity_destroy(&self, entity_destroy: mapped_packet::play::clientbound::EntityDestroy) {
        for id in entity_destroy.entity_ids {
            if let Some(entity) = self.entity_map.write().remove(&id) {
                self.entities.write().world.despawn(entity);
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
        if let Some(entity) = self.entity_map.read().get(&entity_id) {
            let mut entities = self.entities.write();
            let mut entity = entities.world.entity_mut(*entity);
            let mut target_position = entity.get_mut::<TargetPosition>().unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
            let mut target_rotation = entity.get_mut::<TargetRotation>().unwrap();
            target_rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            target_rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_move(&self, entity_move: mapped_packet::play::clientbound::EntityMove) {
        if let Some(entity) = self.entity_map.read().get(&entity_move.entity_id) {
            let mut entities = self.entities.write();
            let mut entity = entities.world.entity_mut(*entity);
            let mut position = entity.get_mut::<TargetPosition>().unwrap();
            position.position.x += entity_move.delta_x;
            position.position.y += entity_move.delta_y;
            position.position.z += entity_move.delta_z;
        }
    }

    fn on_entity_look(&self, entity_id: i32, yaw: f64, pitch: f64) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.read().get(&entity_id) {
            let mut entities = self.entities.write();
            let mut entity = entities.world.entity_mut(*entity);
            let mut rotation = entity.get_mut::<TargetRotation>().unwrap();
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
        if let Some(entity) = self.entity_map.read().get(&entity_id) {
            let mut entities = self.entities.write();
            let mut entity = entities.world.entity_mut(*entity);
            let mut position = entity.get_mut::<TargetPosition>().unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
            let mut rotation = entity.get_mut::<TargetRotation>().unwrap();
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
        if let Some(entity) = self.entity_map.write().remove(&entity_id) {
            self.entities.write().world.despawn(entity);
        }
        let world_entity = entity::player::create_remote(
            &mut self.entities.write(),
            self.players
                .read()
                .get(&uuid)
                .map_or("MISSING", |v| &v.name),
        );
        let mut entities = self.entities.write();
        let mut entity = entities.world.entity_mut(world_entity);
        {
            let mut position = entity.get_mut::<crate::entity::Position>().unwrap();
            position.position.x = x;
            position.position.y = y;
            position.position.z = z;
        }
        {
            let mut target_position = entity.get_mut::<TargetPosition>().unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
        }
        let (yaw, pitch) = {
            let mut rotation = entity.get_mut::<crate::entity::Rotation>().unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
            (rotation.yaw, rotation.pitch)
        };
        {
            let mut target_rotation = entity.get_mut::<TargetRotation>().unwrap();
            target_rotation.yaw = yaw;
            target_rotation.pitch = pitch;
        }
        if let Some(info) = self.players.read().get(&uuid) {
            let mut model = entity.get_mut::<PlayerModel>().unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.entity_map
            .clone()
            .write()
            .insert(entity_id, world_entity);
    }

    fn on_teleport_player(&self, teleport: mapped_packet::play::clientbound::TeleportPlayer) {
        use std::f64::consts::PI;
        if let Some(player) = self.player.load().as_ref() {
            let flags = teleport.flags.unwrap_or(0);
            let mut entities = self.entities.write();
            let mut player_entity = entities.world.entity_mut(player.1);

            let mut position = player_entity.get_mut::<TargetPosition>().unwrap();
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
            let mut rotation = player_entity.get_mut::<crate::entity::Rotation>().unwrap();
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

            let mut velocity = player_entity.get_mut::<crate::entity::Velocity>().unwrap();

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
                        let line1 = format::Component::from_str(
                            nbt.1.get("Text1").unwrap().as_str().unwrap(),
                        );
                        let line2 = format::Component::from_str(
                            nbt.1.get("Text2").unwrap().as_str().unwrap(),
                        );
                        let line3 = format::Component::from_str(
                            nbt.1.get("Text3").unwrap().as_str().unwrap(),
                        );
                        let line4 = format::Component::from_str(
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
        update_sign.line1 = update_sign.line1.try_update_with_legacy();
        update_sign.line2 = update_sign.line2.try_update_with_legacy();
        update_sign.line3 = update_sign.line3.try_update_with_legacy();
        update_sign.line4 = update_sign.line4.try_update_with_legacy();
        self.world
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
                    let mut players = self.players.write();
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
                        let skin_blob_result = &STANDARD.decode(&prop.value);
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
                        let mut entities = self.entities.write();
                        let mut player = entities
                            .world
                            .entity_mut(self.player.load().as_ref().unwrap().1);
                        let mut model = player.get_mut::<entity::player::PlayerModel>().unwrap();
                        model.set_skin(info.skin_url.clone());
                    }
                }
                UpdateGamemode { uuid, gamemode } => {
                    if let Some(info) = self.players.write().get_mut(&uuid) {
                        info.gamemode = GameMode::from_int(gamemode.0);
                    }
                }
                UpdateLatency { uuid, ping } => {
                    if let Some(info) = self.players.write().get_mut(&uuid) {
                        info.ping = ping.0;
                    }
                }
                UpdateDisplayName { uuid, display } => {
                    if let Some(info) = self.players.write().get_mut(&uuid) {
                        info.display_name = display;
                    }
                }
                Remove { uuid } => {
                    self.players.write().remove(&uuid);
                }
            }
        }
    }

    fn on_servermessage(&self, message: mapped_packet::play::clientbound::ServerMessage) {
        debug!("Received chat message: {}", message.message);
        self.hud_context
            .write()
            .display_message_in_chat(message.message);
        self.received_chat_at.store(Some(Arc::new(Instant::now())));
    }

    fn load_block_entities(&self, block_entities: Vec<Option<crate::nbt::NamedTag>>) {
        for block_entity in block_entities.into_iter().flatten() {
            let x = block_entity.1.get("x").unwrap().as_int().unwrap();
            let y = block_entity.1.get("y").unwrap().as_int().unwrap();
            let z = block_entity.1.get("z").unwrap().as_int().unwrap();
            if let Some(tile_id) = block_entity.1.get("id") {
                let tile_id = tile_id.as_str().unwrap();
                let action = match tile_id {
                    // Fake a sign update
                    "Sign" => 9,
                    // Not something we care about, so break the loop
                    _ => continue,
                };
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
        sky_light: bool,
    ) {
        self.world
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_biomes3d_bool(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_Biomes3D_bool,
        sky_light: bool,
    ) {
        self.world
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_biomes3d(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_Biomes3D,
        sky_light: bool,
    ) {
        self.world
            .load_chunk115(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData,
        sky_light: bool,
    ) {
        self.world
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_heightmap(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_HeightMap,
        sky_light: bool,
    ) {
        self.world
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
                chunk_data.bitmask as u16,
                chunk_data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities);
    }

    fn on_chunk_data_no_entities(
        &self,
        chunk_data: mapped_packet::play::clientbound::ChunkData_NoEntities,
        sky_light: bool,
    ) {
        self.world
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                sky_light,
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
            .load_chunks18(chunk_data.new, skylight, &chunk_meta, chunk_data.data)
            .unwrap();
    }

    fn on_chunk_data_17(&self, chunk_data: mapped_packet::play::clientbound::ChunkData_17) {
        self.world
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
            .load_chunks17(
                bulk.chunk_column_count,
                bulk.data_length,
                bulk.skylight,
                &bulk.chunk_data_and_meta,
            )
            .unwrap();
    }

    fn on_chunk_unload(&self, chunk_unload: mapped_packet::play::clientbound::ChunkUnload) {
        self.world
            .unload_chunk(chunk_unload.x, chunk_unload.z, &mut self.entities.write());
    }

    fn on_block_change_in_world(&self, location: Position, id: i32) {
        let block = self
            .world
            .id_map
            .by_vanilla_id(id as usize, &self.world.modded_block_ids.load());
        self.world.set_block(location, block)
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
                record.block_id,
            );
        }
    }

    pub fn on_update_health(&self, health: f32, food: u8, saturation: u8) {
        self.hud_context
            .write()
            .update_health_and_food(health, food, saturation);
        if health <= 0.0 && !self.dead.load(Ordering::Acquire) {
            self.dead.store(true, Ordering::Release);
            self.screen_sys.close_closable_screens();
            self.screen_sys.add_screen(Box::new(Respawn::new(0))); // TODO: Use the correct score!
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

#[derive(Resource)]
pub struct WorldResource(pub Arc<World>);

#[derive(Resource)]
pub struct RendererResource(pub Arc<Renderer>);

#[derive(Resource)]
pub struct ScreenSystemResource(pub Arc<ScreenSystem>);

#[derive(Resource)]
pub struct ConnResource(pub Arc<RwLock<Option<Conn>>>);

#[derive(Resource)]
pub struct InventoryContextResource(pub Arc<RwLock<InventoryContext>>);

#[derive(Resource)]
pub struct RenderCtxResource(pub Arc<RenderCtx>);

impl Default for RenderCtxResource {
    fn default() -> Self {
        Self(Arc::new(RenderCtx {
            fps: AtomicU32::new(0),
            frame_start: AtomicU64::new(0),
        }))
    }
}

pub struct RenderCtx {
    pub fps: AtomicU32,
    pub frame_start: AtomicU64,
}

#[derive(Resource)]
pub struct SunModelResource(pub SunModel);

#[derive(Resource)]
pub struct TargetResource(pub Arc<RwLock<target::Info>>);

#[derive(Resource)]
pub struct DeltaResource(pub f64);

fn tick_sun(
    mut sun: ResMut<SunModelResource>,
    renderer: Res<RendererResource>,
    world_data: ResMut<WorldData>,
) {
    sun.0.tick(
        renderer.0.clone(),
        world_data.world_time,
        world_data.world_age,
    );
}

fn tick_world(mut commands: Commands, world: Res<WorldResource>) {
    world.0.tick(&mut commands);
}

fn tick_time(mut world_data: ResMut<WorldData>, delta: Res<DeltaResource>) {
    let delta = delta.0;
    if world_data.tick_time {
        world_data.world_time_target += delta / 3.0;
        let time = world_data.world_time_target;
        world_data.world_time_target = (24000.0 + time) % 24000.0;
        let mut diff = world_data.world_time_target - world_data.world_time;
        if diff < -12000.0 {
            diff += 24000.0
        } else if diff > 12000.0 {
            diff -= 24000.0
        }
        world_data.world_time += diff * (1.5 / 60.0) * delta;
        let time = world_data.world_time;
        world_data.world_time = (24000.0 + time) % 24000.0;
    } else {
        let time = world_data.world_time_target;
        world_data.world_time = time;
    }
}

fn add_systems(sched: &mut Schedule) {
    sched
        .add_systems(tick_sun.in_set(SystemExecStage::Render))
        .add_systems(tick_world.in_set(SystemExecStage::Normal))
        .add_systems(tick_time.in_set(SystemExecStage::Normal));
}
