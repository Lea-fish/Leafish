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

use crate::ecs;
use crate::entity;
use crate::format;
use crate::protocol::{self, forge, mojang, packet};
use crate::render;
use crate::resources;
use crate::settings::Actionkey;
use crate::shared::{Axis, Position};
use crate::types::hash::FNVHash;
use crate::types::Gamemode;
use crate::world;
use crate::world::{block, World};
use cgmath::prelude::*;
use instant::{Instant, Duration};
use log::{debug, error, info, warn};
use rand::{self, Rng};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::str::FromStr;
use std::sync::{mpsc, Mutex};
use std::sync::{Arc, RwLock};
use std::thread;
use leafish_protocol::protocol::packet::Packet;
use std::sync::mpsc::Sender;
use std::io::{Write, Cursor};
use leafish_protocol::protocol::{VarInt, Serializable, UUID, Conn};
use crate::entity::{TargetPosition, TargetRotation};
use crate::ecs::{Key, Manager};
use crate::entity::player::PlayerMovement;
use std::thread::sleep;
use std::ops::{Add, Sub};
use rayon::prelude::*;
use leafish_protocol::protocol::packet::Packet::ConfirmTransactionServerbound;

pub mod plugin_messages;
mod sun;
pub mod target;

#[derive(Default)]
pub struct DisconnectData {

    pub disconnect_reason: Option<format::Component>, // remove somehow! (interior mut?)
    just_disconnected: bool, // remove somehow! (interior mut?)

}

struct WorldData {

    world_age: i64, // move to world?
    world_time: f64, // move to world?
    world_time_target: f64, // move to world?
    tick_time: bool, // move to world?

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
    uuid: protocol::UUID, // const
    conn: Arc<RwLock<Option<protocol::Conn>>>, // 'const'
    protocol_version: i32, // const
    forge_mods: Vec<forge::ForgeMod>, // ?
    // read_queue: Mutex<Option<mpsc::Receiver<Result<packet::Packet, protocol::Error>>>>, // move to conn
    // write_queue: Mutex<Option<mpsc::Sender<(i32, bool, Vec<u8>)>>>, // move to conn
    pub disconnect_data: Arc<RwLock<DisconnectData>>,

    pub world: Arc<world::World>,
    pub entities: Arc<RwLock<ecs::Manager>>,
    world_data: Arc<RwLock<WorldData>>,

    resources: Arc<RwLock<resources::Manager>>,
    version: RwLock<Option<usize>>,

    // Entity accessors
    game_info: ecs::Key<entity::GameInfo>, // const!
    player_movement: ecs::Key<entity::player::PlayerMovement>, // const! (rem arc)
    gravity: ecs::Key<entity::Gravity>, // const!
    position: ecs::Key<entity::Position>, // const! (rem arc)
    target_position: ecs::Key<entity::TargetPosition>, // const! (rem arc)
    velocity: ecs::Key<entity::Velocity>, // const! (rem arc)
    gamemode: ecs::Key<Gamemode>, // const! (rem arc)
    pub rotation: ecs::Key<entity::Rotation>, // const! (rem arc)
    target_rotation: ecs::Key<entity::TargetRotation>, // const! (rem arc)
    //
    pub player: Arc<RwLock<Option<ecs::Entity>>>,
    entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
    players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,

    tick_timer: RwLock<f64>,
    entity_tick_timer: RwLock<f64>,
    pub received_chat_at: Arc<RwLock<Option<Instant>>>,

    sun_model: RwLock<Option<sun::SunModel>>,
    target_info: Arc<RwLock<target::Info>>,
    pub light_updates: Mutex<Sender<bool>>, // move to world!
}

#[derive(Debug)]
pub struct PlayerInfo {
    name: String,
    uuid: protocol::UUID,
    skin_url: Option<String>,

    display_name: Option<format::Component>,
    ping: i32,
    gamemode: Gamemode,
}

macro_rules! handle_packet {
    ($s:ident $pck:ident {
        $($packet:ident => $func:ident,)*
    }) => (
        match $pck {
        $(
            protocol::packet::Packet::$packet(val) => $s.$func(val),
        )*
            _ => {},
        }
    )
}

impl Server {
    pub fn connect(
        resources: Arc<RwLock<resources::Manager>>,
        profile: mojang::Profile,
        address: &str,
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        fml_network_version: Option<i64>,
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
                    conn.set_compresssion(val.threshold.0);
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
                    //let mut read = conn.clone();
                    //let write_int = conn.clone();
                    //let mut write = conn;
                    //read.state = protocol::State::Play;
                    //write.state = protocol::State::Play;
                    conn.state = protocol::State::Play;
                    let uuid = protocol::UUID::from_str(&val.uuid).unwrap();
                    let server = Server::connect0(conn/*read*/, protocol_version, forge_mods, uuid, resources/*, write, write_int*/);
                    return Ok(server);
                }
                protocol::packet::Packet::LoginSuccess_UUID(val) => {
                    warn!("Server is running in offline mode");
                    debug!("Login: {} {:?}", val.username, val.uuid);
                    // let mut read = conn.clone();
                    // let write_int = conn.clone();
                    // let mut write = conn;
                    // read.state = protocol::State::Play;
                    // write.state = protocol::State::Play;
                    conn.state = protocol::State::Play;
                    let server = Server::connect0(conn/*read*/, protocol_version, forge_mods, val.uuid, resources/*, write, write_int*/);

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
        //let mut write = conn;

        conn/*write*/.enable_encyption(&shared/*, false*/);
        // let mut read = write.clone();
        // let mut write_int = write.clone();

        let uuid;
        let compression_threshold = conn/*read*/.compression_threshold;
        loop {
            match conn/*read*/.read_packet()? {
                protocol::packet::Packet::SetInitialCompression(val) => {
                    // read.set_compresssion(val.threshold.0);
                    // write.set_compresssion(val.threshold.0);
                    // write_int.set_compresssion(val.threshold.0);
                    conn.set_compresssion(val.threshold.0);
                }
                protocol::packet::Packet::LoginSuccess_String(val) => {
                    debug!("Login: {} {}", val.username, val.uuid);
                    uuid = protocol::UUID::from_str(&val.uuid).unwrap();
                    // read.state = protocol::State::Play;
                    // write.state = protocol::State::Play;
                    // write_int.state = protocol::State::Play;
                    conn.state = protocol::State::Play;
                    break;
                }
                protocol::packet::Packet::LoginSuccess_UUID(val) => {
                    debug!("Login: {} {:?}", val.username, val.uuid);
                    uuid = val.uuid;
                    // read.state = protocol::State::Play;
                    // write.state = protocol::State::Play;
                    // write_int.state = protocol::State::Play;
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

        let server = Server::connect0(conn/*read*/, protocol_version, forge_mods, uuid, resources/*, write, write_int*/);

        Ok(server)
    }

    fn connect0(conn: Conn, protocol_version: i32,
                forge_mods: Vec<forge::ForgeMod>,
                uuid: protocol::UUID,
                resources: Arc<RwLock<resources::Manager>>,
                // write: Conn,
                /*write_int: Conn*/) -> Arc<Server> {
        // let tx = Self::spawn_writer(write_int);
        // read.send = Some(tx.clone());
        let server_callback = Arc::new(RwLock::new(None));
        let inner_server = server_callback.clone();
        let mut inner_server = inner_server.write().unwrap();
        Self::spawn_reader(conn.clone(), protocol_version, uuid.clone(), server_callback.clone());
        let light_updater = Self::spawn_light_updater(server_callback.clone());
        let conn = Arc::new(RwLock::new(Some(conn)));
        let server = Arc::new(Server::new(
            protocol_version,
            forge_mods,
            uuid,
            resources,
            conn,
            /*Some(rx),
            Some(tx),*/
            light_updater
        ));

        let actual_server = server.clone();
        inner_server.replace(actual_server);
        server.clone()
    }

    fn spawn_reader(
        mut read: protocol::Conn,
        protocol_version: i32,
        uuid: UUID,
        server: Arc<RwLock<Option<Arc<Server>>>>,
    ) {
        thread::spawn(move || loop {
            while server.clone().try_read().is_err() {
            }
            let server = server.clone().read().unwrap().as_ref().unwrap().clone();
            let pck = read.read_packet();
                match pck {
                    Ok(pck) =>
                        match pck {
                            Packet::KeepAliveClientbound_i64(keep_alive) => {
                                println!("pre keep alive!");
                                read.write_packet(packet::play::serverbound::KeepAliveServerbound_i64 {
                                    id: keep_alive.id,
                                }).unwrap();
                                println!("keep alive!");
                            },
                            Packet::KeepAliveClientbound_VarInt(keep_alive) => {
                                println!("pre keep alive!");
                                read.write_packet(packet::play::serverbound::KeepAliveServerbound_VarInt {
                                    id: keep_alive.id,
                                }).unwrap();
                                println!("keep alive!");
                            },
                            Packet::KeepAliveClientbound_i32(keep_alive) => {
                                println!("pre keep alive!");
                                read.write_packet(packet::play::serverbound::KeepAliveServerbound_i32 {
                                    id: keep_alive.id,
                                }).unwrap();
                                println!("keep alive!");
                            },
                            Packet::ChunkData_NoEntities(chunk_data) => {
                                server.world.clone()
                                    .load_chunk19(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                            },
                            Packet::ChunkData_NoEntities_u16(chunk_data) => {
                                let chunk_meta = vec![crate::protocol::packet::ChunkMeta {
                                    x: chunk_data.chunk_x,
                                    z: chunk_data.chunk_z,
                                    bitmask: chunk_data.bitmask,
                                }];
                                let skylight = false;
                                server.world.clone()
                                    .load_chunks18(chunk_data.new, skylight, &chunk_meta, chunk_data.data.data)
                                    .unwrap();
                            },
                            Packet::ChunkData_17(chunk_data) => {
                                server.world.clone()
                                    .load_chunk17(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask,
                                        chunk_data.add_bitmask,
                                        chunk_data.compressed_data.data,
                                    )
                                    .unwrap();
                            },
                            Packet::ChunkDataBulk(chunk_data) => {
                                let new = true;
                                server.world.clone()
                                    .load_chunks18(
                                        new,
                                        chunk_data.skylight,
                                        &chunk_data.chunk_meta.data,
                                        chunk_data.chunk_data.to_vec(),
                                    )
                                    .unwrap();
                            },
                            Packet::ChunkDataBulk_17(bulk) => {
                                server.world.clone()
                                    .load_chunks17(
                                        bulk.chunk_column_count,
                                        bulk.data_length,
                                        bulk.skylight,
                                        &bulk.chunk_data_and_meta,
                                    )
                                    .unwrap();
                            },
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(server.world.clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::BlockChange_u8(block_change) => {
                                Server::on_block_change_in_world(server.world.clone(),
                                                                 crate::shared::Position::new(block_change.x, block_change.y as i32, block_change.z),
                                                                 (block_change.block_id.0 << 4) | (block_change.block_metadata as i32));
                            },
                            Packet::MultiBlockChange_Packed(block_change) => {
                                let sx = (block_change.chunk_section_pos >> 42) as i32;
                                let sy = ((block_change.chunk_section_pos << 44) >> 44) as i32;
                                let sz = ((block_change.chunk_section_pos << 22) >> 42) as i32;

                                for record in block_change.records.data {
                                    let block_raw_id = record.0 >> 12;
                                    let lz = (record.0 & 0xf) as i32;
                                    let ly = ((record.0 >> 4) & 0xf) as i32;
                                    let lx = ((record.0 >> 8) & 0xf) as i32;

                                    /*
                                    let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
                                    let block = self.world.clone().read().unwrap()
                                        .id_map
                                        .by_vanilla_id(block_raw_id as usize, modded_block_ids);
                                    self.world.clone().write().unwrap().set_block(
                                        Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32),
                                        block,
                                    );*/
                                    Server::on_block_change_in_world(server.world.clone(), Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32), block_raw_id as i32);
                                }
                            },
                            Packet::MultiBlockChange_VarInt(block_change) => {
                                let ox = block_change.chunk_x << 4;
                                let oz = block_change.chunk_z << 4;
                                for record in block_change.records.data {
                                    /*let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
                                    let block = self.world.clone().read().unwrap()
                                        .id_map
                                        .by_vanilla_id(record.block_id.0 as usize, modded_block_ids);
                                    self.world.clone().write().unwrap().set_block(
                                        Position::new(
                                            ox + (record.xz >> 4) as i32,
                                            record.y as i32,
                                            oz + (record.xz & 0xF) as i32,
                                        ),
                                        block,
                                    );*/
                                    Server::on_block_change_in_world(server.world.clone(), Position::new(
                                        ox + (record.xz >> 4) as i32,
                                        record.y as i32,
                                        oz + (record.xz & 0xF) as i32,
                                    ), record.block_id.0 as i32);
                                }
                            },
                            Packet::MultiBlockChange_u16(block_change) => {
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
                                    let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
                                    let block = self.world.clone().read().unwrap()
                                        .id_map
                                        .by_vanilla_id(id as usize, modded_block_ids);
                                    self.world.write().unwrap().set_block(
                                        Position::new(x, y, z),
                                        block,
                                    );*/
                                    Server::on_block_change_in_world(server.world.clone(), Position::new(x, y, z), id as i32);
                                }
                            },
                            Packet::UpdateBlockEntity(block_update) => {
                                match block_update.nbt {
                                    None => {
                                        // NBT is null, so we need to remove the block entity
                                        server.world.clone()
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
                                                server.world.clone()
                                                    .add_block_entity_action(
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
                            },
                            Packet::ChunkData_Biomes3D(chunk_data) => {
                                // println!("data x {} z {}", chunk_data.chunk_x, chunk_data.chunk_z);
                                server.world.clone()
                                    .load_chunk115(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                                Server::load_block_entities_glob(server.world.clone(), chunk_data.block_entities.data);
                            },
                            Packet::ChunkData_Biomes3D_VarInt(chunk_data) => {
                                server.world.clone()
                                    .load_chunk115(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                                Server::load_block_entities_glob(server.world.clone(), chunk_data.block_entities.data);
                            },
                            Packet::ChunkData_Biomes3D_bool(chunk_data) => {
                                server.world.clone()
                                    .load_chunk115(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                                Server::load_block_entities_glob(server.world.clone(), chunk_data.block_entities.data);
                            },
                            Packet::ChunkData(chunk_data) => {
                                server.world.clone()
                                    .load_chunk19(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                                Server::load_block_entities_glob(server.world.clone(), chunk_data.block_entities.data);
                            },
                            Packet::ChunkData_HeightMap(chunk_data) => {
                                server.world.clone()
                                    .load_chunk19(
                                        chunk_data.chunk_x,
                                        chunk_data.chunk_z,
                                        chunk_data.new,
                                        chunk_data.bitmask.0 as u16,
                                        chunk_data.data.data,
                                    )
                                    .unwrap();
                                Server::load_block_entities_glob(server.world.clone(), chunk_data.block_entities.data);
                            },
                            Packet::UpdateSign(mut update_sign) => {
                                format::convert_legacy(&mut update_sign.line1);
                                format::convert_legacy(&mut update_sign.line2);
                                format::convert_legacy(&mut update_sign.line3);
                                format::convert_legacy(&mut update_sign.line4);
                                server.world.clone()
                                    .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                                        update_sign.location,
                                        update_sign.line1,
                                        update_sign.line2,
                                        update_sign.line3,
                                        update_sign.line4,
                                    ))));
                            },
                            Packet::UpdateSign_u16(mut update_sign) => {
                                format::convert_legacy(&mut update_sign.line1);
                                format::convert_legacy(&mut update_sign.line2);
                                format::convert_legacy(&mut update_sign.line3);
                                format::convert_legacy(&mut update_sign.line4);
                                server.world.clone()
                                    .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                                        Position::new(update_sign.x, update_sign.y as i32, update_sign.z),
                                        update_sign.line1,
                                        update_sign.line2,
                                        update_sign.line3,
                                        update_sign.line4,
                                    ))));
                            },
                            Packet::UpdateBlockEntity_Data(chunk_data) => {
                                // TODO: handle UpdateBlockEntity_Data for 1.7, decompress gzipped_nbt
                            },
                            Packet::ChunkUnload(chunk_unload) => {
                                server.world.clone()
                                    .unload_chunk(chunk_unload.x, chunk_unload.z, &mut server.entities.clone().write().unwrap());
                            },
                            Packet::EntityDestroy(entity_destroy) => {
                                for id in entity_destroy.entity_ids.data {
                                    if let Some(entity) = server.entity_map.clone().write().unwrap().remove(&id.0) {
                                        server.entities.clone().write().unwrap().remove_entity(entity);
                                    }
                                }
                            },
                            Packet::EntityDestroy_u8(entity_destroy) => {
                                for id in entity_destroy.entity_ids.data {
                                    if let Some(entity) = server.entity_map.clone().write().unwrap().remove(&id) {
                                        server.entities.clone().write().unwrap().remove_entity(entity);
                                    }
                                }
                            },
                            /*
                            Packet::PlayerInfo(player_info) => {
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
                                            let players = players.clone().lock().unwrap().as_ref().unwrap().clone();
                                            let mut players = players.write().unwrap();
                                            let info = players.entry(uuid.clone()).or_insert(PlayerInfo {
                                                name: name.clone(),
                                                uuid,
                                                skin_url: None,

                                                display_name: display.clone(),
                                                ping: ping.0,
                                                gamemode: Gamemode::from_int(gamemode.0),
                                            });
                                            // Re-set the props of the player in case of dodgy server implementations
                                            info.name = name;
                                            info.display_name = display;
                                            info.ping = ping.0;
                                            info.gamemode = Gamemode::from_int(gamemode.0);
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
                                                let skin_blob: serde_json::Value = match serde_json::from_slice(&skin_blob)
                                                {
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
                                            if info.uuid == uuid {
                                                let model = entities
                                                    .clone().lock().unwrap().as_ref().unwrap().clone().write().unwrap()
                                                    .get_component_mut_direct::<entity::player::PlayerModel>(
                                                        self.player.unwrap(),
                                                    )
                                                    .unwrap();
                                                model.set_skin(info.skin_url.clone());
                                            }
                                        }
                                        UpdateGamemode { uuid, gamemode } => {
                                            if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                                                info.gamemode = Gamemode::from_int(gamemode.0);
                                            }
                                        }
                                        UpdateLatency { uuid, ping } => {
                                            if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                                                info.ping = ping.0;
                                            }
                                        }
                                        UpdateDisplayName { uuid, display } => {
                                            if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                                                info.display_name = display;
                                            }
                                        }
                                        Remove { uuid } => {
                                            self.players.clone().write().unwrap().remove(&uuid);
                                        }
                                    }
                                }
                            },*/
                            Packet::EntityMove_i8_i32_NoGround(m) => {
                                Server::on_entity_move_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    m.entity_id,
                                    f64::from(m.delta_x),
                                    f64::from(m.delta_y),
                                    f64::from(m.delta_z),
                                )
                            },
                            Packet::EntityMove_i8(m) => {
                                Server::on_entity_move_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    m.entity_id.0,
                                    f64::from(m.delta_x),
                                    f64::from(m.delta_y),
                                    f64::from(m.delta_z),
                                )
                            },
                            Packet::EntityMove_i16(m) => {
                                Server::on_entity_move_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    m.entity_id.0,
                                    f64::from(m.delta_x),
                                    f64::from(m.delta_y),
                                    f64::from(m.delta_z),
                                )
                            },
                            Packet::EntityLook_VarInt(look) => {
                                Server::on_entity_look_glob(
                                    server.entity_map.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    look.entity_id.0, look.yaw as f64, look.pitch as f64)
                            },
                            Packet::EntityLook_i32_NoGround(look) => {
                                Server::on_entity_look_glob(
                                    server.entity_map.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    look.entity_id, look.yaw as f64, look.pitch as f64)
                            },
                            Packet::JoinGame_HashedSeed_Respawn(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                &uuid,
                                protocol_version,
                                                          &mut read,
                                join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_i8(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_i8_NoDebug(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone().clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_i32(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone().clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_i32_ViewDistance(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone().clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_WorldNames(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone().clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::JoinGame_WorldNames_IsHard(join) => {
                                Server::on_game_join_glob(server.players.clone(),
                                                          server.player.clone(),
                                                          server.entity_map.clone(),
                                                          server.player_movement.clone().clone(),
                                                          server.entities.clone(),
                                                          server.gamemode.clone(),
                                                          &uuid,
                                                          protocol_version,
                                                          &mut read,
                                                          join.gamemode, join.entity_id);
                            },
                            Packet::TeleportPlayer_WithConfirm(teleport) => {
                                Server::on_teleport_player_glob(
                                    server.player.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    server.velocity.clone(),
                                    server.rotation.clone(),
                                    &mut read,
                                    teleport.x,
                                    teleport.y,
                                    teleport.z,
                                    teleport.yaw as f64,
                                    teleport.pitch as f64,
                                    teleport.flags,
                                    Some(teleport.teleport_id),
                                )
                            },
                            Packet::TeleportPlayer_NoConfirm(teleport) => {
                                Server::on_teleport_player_glob(
                                    server.player.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    server.velocity.clone(),
                                    server.rotation.clone(),
                                    &mut read,
                                    teleport.x,
                                    teleport.y,
                                    teleport.z,
                                    teleport.yaw as f64,
                                    teleport.pitch as f64,
                                    teleport.flags,
                                    None,
                                )
                            },
                            Packet::TeleportPlayer_OnGround(teleport) => {
                                let flags: u8 = 0; // always absolute
                                Server::on_teleport_player_glob(
                                    server.player.clone(),
                                    server.target_position.clone(),
                                    server.entities.clone(),
                                    server.velocity.clone(),
                                    server.rotation.clone(),
                                    &mut read,
                                    teleport.x,
                                    teleport.eyes_y - 1.62,
                                    teleport.z,
                                    teleport.yaw as f64,
                                    teleport.pitch as f64,
                                    flags,
                                    None,
                                )
                            },
                            Packet::Respawn_Gamemode(respawn) => {
                                Server::respawn_glob(
                                    server.world.clone(),
                                    server.player.clone(),
                                    server.player_movement.clone(),
                                    server.entities.clone(),
                                    server.gamemode.clone(),
                                    protocol_version,
                                    respawn.gamemode)
                            },
                            Packet::Respawn_HashedSeed(respawn) => {
                                Server::respawn_glob(
                                    server.world.clone(),
                                    server.player.clone(),
                                    server.player_movement.clone().clone(),
                                    server.entities.clone(),
                                    server.gamemode.clone(),
                                    protocol_version,
                                    respawn.gamemode)
                            },
                            Packet::Respawn_NBT(respawn) => {
                                Server::respawn_glob(
                                    server.world.clone(),
                                    server.player.clone(),
                                    server.player_movement.clone().clone(),
                                    server.entities.clone(),
                                    server.gamemode.clone(),
                                    protocol_version,
                                    respawn.gamemode)
                            },
                            Packet::Respawn_WorldName(respawn) => {
                                Server::respawn_glob(
                                    server.world.clone(),
                                    server.player.clone(),
                                    server.player_movement.clone().clone(),
                                    server.entities.clone(),
                                    server.gamemode.clone(),
                                    server.protocol_version,
                                    respawn.gamemode)
                            },
                            Packet::EntityTeleport_f64(entity_teleport) => {
                                Server::on_entity_teleport_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    entity_teleport.entity_id.0,
                                    entity_teleport.x,
                                    entity_teleport.y,
                                    entity_teleport.z,
                                    entity_teleport.yaw as f64,
                                    entity_teleport.pitch as f64,
                                    entity_teleport.on_ground,
                                )
                            },
                            Packet::EntityTeleport_i32(entity_teleport) => {
                                Server::on_entity_teleport_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    entity_teleport.entity_id.0,
                                    f64::from(entity_teleport.x),
                                    f64::from(entity_teleport.y),
                                    f64::from(entity_teleport.z),
                                    entity_teleport.yaw as f64,
                                    entity_teleport.pitch as f64,
                                    entity_teleport.on_ground,
                                )
                            },
                            Packet::EntityTeleport_i32_i32_NoGround(entity_teleport) => {
                                let on_ground = true; // TODO: how is this supposed to be set? (for 1.7)
                                Server::on_entity_teleport_glob(
                                    server.entity_map.clone(),
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    entity_teleport.entity_id,
                                    f64::from(entity_teleport.x),
                                    f64::from(entity_teleport.y),
                                    f64::from(entity_teleport.z),
                                    entity_teleport.yaw as f64,
                                    entity_teleport.pitch as f64,
                                    on_ground,
                                )
                            },
                            Packet::EntityLookAndMove_i8_i32_NoGround(lookmove) => {
                                Server::on_entity_look_and_move_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    lookmove.entity_id,
                                    f64::from(lookmove.delta_x),
                                    f64::from(lookmove.delta_y),
                                    f64::from(lookmove.delta_z),
                                    lookmove.yaw as f64,
                                    lookmove.pitch as f64,
                                )
                            },
                            Packet::EntityLookAndMove_i8(lookmove) => {
                                Server::on_entity_look_and_move_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    lookmove.entity_id.0,
                                    f64::from(lookmove.delta_x),
                                    f64::from(lookmove.delta_y),
                                    f64::from(lookmove.delta_z),
                                    lookmove.yaw as f64,
                                    lookmove.pitch as f64,
                                )
                            },
                            Packet::EntityLookAndMove_i16(lookmove) => {
                                Server::on_entity_look_and_move_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    lookmove.entity_id.0,
                                    f64::from(lookmove.delta_x),
                                    f64::from(lookmove.delta_y),
                                    f64::from(lookmove.delta_z),
                                    lookmove.yaw as f64,
                                    lookmove.pitch as f64,
                                )
                            },
                            Packet::SpawnPlayer_i32_HeldItem_String(spawn) => {
                                // 1.7.10: populate the player list here, since we only now know the UUID
                                let uuid = protocol::UUID::from_str(&spawn.uuid).unwrap();
                                server.players.clone().write().unwrap().entry(uuid.clone()).or_insert(PlayerInfo {
                                    name: spawn.name.clone(),
                                    uuid,
                                    skin_url: None,

                                    display_name: None,
                                    ping: 0, // TODO: don't overwrite from PlayerInfo_String
                                    gamemode: Gamemode::from_int(0),
                                });

                                Server::on_player_spawn_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    server.players.clone(),
                                    server.position.clone(),
                                    server.rotation.clone(),
                                    spawn.entity_id.0,
                                    protocol::UUID::from_str(&spawn.uuid).unwrap(),
                                    f64::from(spawn.x),
                                    f64::from(spawn.y),
                                    f64::from(spawn.z),
                                    spawn.yaw as f64,
                                    spawn.pitch as f64,
                                )
                            },
                            Packet::SpawnPlayer_i32_HeldItem(spawn) => {
                                Server::on_player_spawn_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    server.players.clone(),
                                    server.position.clone(),
                                    server.rotation.clone(),
                                    spawn.entity_id.0,
                                    spawn.uuid,
                                    f64::from(spawn.x),
                                    f64::from(spawn.y),
                                    f64::from(spawn.z),
                                    spawn.yaw as f64,
                                    spawn.pitch as f64,
                                )
                            },
                            Packet::SpawnPlayer_i32(spawn) => {
                                Server::on_player_spawn_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    server.players.clone(),
                                    server.position.clone(),
                                    server.rotation.clone(),
                                    spawn.entity_id.0,
                                    spawn.uuid,
                                    f64::from(spawn.x),
                                    f64::from(spawn.y),
                                    f64::from(spawn.z),
                                    spawn.yaw as f64,
                                    spawn.pitch as f64,
                                )
                            },
                            Packet::SpawnPlayer_f64(spawn) => {
                                Server::on_player_spawn_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    server.players.clone(),
                                    server.position.clone(),
                                    server.rotation.clone(),
                                    spawn.entity_id.0,
                                    spawn.uuid,
                                    spawn.x,
                                    spawn.y,
                                    spawn.z,
                                    spawn.yaw as f64,
                                    spawn.pitch as f64,
                                )
                            },
                            Packet::SpawnPlayer_f64_NoMeta(spawn) => {
                                Server::on_player_spawn_glob(
                                    server.target_position.clone(),
                                    server.target_rotation.clone(),
                                    server.entities.clone(),
                                    server.entity_map.clone(),
                                    server.players.clone(),
                                    server.position.clone(),
                                    server.rotation.clone(),
                                    spawn.entity_id.0,
                                    spawn.uuid,
                                    spawn.x,
                                    spawn.y,
                                    spawn.z,
                                    spawn.yaw as f64,
                                    spawn.pitch as f64,
                                )
                            },

                            Packet::PlayerInfo(player_info) => {
                                Server::on_player_info_glob(
                                    server.entities.clone(),
                                    server.players.clone(),
                                    server.player.clone(),
                                    uuid.clone(),
                                    player_info
                                )
                                // Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::ConfirmTransaction(transaction) => {
                                read.write_packet(packet::play::serverbound::ConfirmTransactionServerbound {
                                    id: 0, // TODO: Use current container id, if the id of the transaction is not 0.
                                    action_number: transaction.action_number,
                                    accepted: true,
                                });
                            },
                            // unknown: 37, 23, 50, 60, 70, 68, 89, 76
                            Packet::UpdateLight_NoTrust(update_light) => { // 37 (1.15.2)
                                server.world.clone().load_light_with_loc(update_light.chunk_x.0, update_light.chunk_z.0,
                                                                         update_light.block_light_mask.0, true,
                                                                         update_light.sky_light_mask.0, &mut Cursor::new(update_light.light_arrays));
                            },
                            Packet::UpdateLight_WithTrust(update_light) => {
                                // TODO: Add specific stuff!
                                server.world.clone().load_light_with_loc(update_light.chunk_x.0, update_light.chunk_z.0,
                                                                         update_light.block_light_mask.0, true,
                                                                         update_light.sky_light_mask.0, &mut Cursor::new(update_light.light_arrays));
                            },
                            /*
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },
                            Packet::BlockChange_VarInt(block_change) => {
                                Server::on_block_change_in_world(world.clone().lock().unwrap().as_ref().unwrap().clone(), block_change.location, block_change.block_id.0);
                            },*/
                            Packet::PluginMessageClientbound_i16(plugin_message) => {
                                Server::on_plugin_message_clientbound_i16_glob(server.clone(), plugin_message);
                            },
                            Packet::PluginMessageClientbound(plugin_message) => {
                                Server::on_plugin_message_clientbound_1_glob(server.clone(), plugin_message);
                            },
                            _ => {
                                println!("other packet!");
                            }
                        },
                    Err(err) => {
                        panic!("An error occurred while reading a packet!");
                    },
                }
        });
    }

    /*
    fn spawn_writer(
        mut write: protocol::Conn
    ) -> Sender<(i32, bool, Vec<u8>)> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            let send: (i32, bool, Vec<u8>) = rx.recv().unwrap();
            println!("sending...!");
            let process_buffer = send.2;
            VarInt(process_buffer.len() as i32 + send.0).write_to(&mut write).unwrap()/*?*/;
            if send.1 && send.0 == 1 {
                VarInt(0).write_to(&mut write).unwrap()/*?*/;
            }
            write.write_all(&process_buffer).expect("Failed to write data to the connection!");
        });
        tx
    }*/

    fn spawn_light_updater(server: Arc<RwLock<Option<Arc<Server>>>>) -> Sender<bool> { // TODO: Use fair rwlock!
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || loop {
            rx.recv().unwrap();
            while server.clone().try_read().is_err() {
            }
            let server = server.clone().read().unwrap().as_ref().unwrap().clone();
            let mut done = false;
            while !done {
                let start = Instant::now();
                let mut updates_performed = 0;
                let world_cloned = server.world.clone();
                while !world_cloned.light_updates.clone().read().unwrap().is_empty() {
                    updates_performed += 1;
                    world_cloned.do_light_update();
                    if (updates_performed & 0xFFF == 0) && start.elapsed().subsec_nanos() >= 5000000 {
                        // 5 ms for light updates
                        break;
                    }
                }
                if world_cloned.light_updates.clone().read().unwrap().is_empty() {
                    done = true;
                }
                sleep(Duration::from_millis(1));
            }
            while rx.try_recv().is_ok() {}
        });
        tx
    }

    /*
    Diff7 took 1
[model/mod.rs:226][ERROR] Error missing block state for minecraft:bed
[model/mod.rs:196][ERROR] Error loading model minecraft:bed
thread '<unnamed>' panicked at 'called `Result::unwrap()` on an `Err` value: RecvError', src/server/mod.rs:1307:56
stack backtrace:
   0: rust_begin_unwind
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/std/src/panicking.rs:515:5
   1: core::panicking::panic_fmt
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/panicking.rs:92:14
   2: core::result::unwrap_failed
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/result.rs:1355:5
   3: core::result::Result<T,E>::unwrap
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/result.rs:1037:23
   4: leafish::server::Server::spawn_writer::{{closure}}
             at /home/threadexception/IdeaProjects/Leafish/src/server/mod.rs:1307:46
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

Process finished with exit code 137 (interrupted by signal 9: SIGKILL)
*/
    pub fn dummy_server(resources: Arc<RwLock<resources::Manager>>) -> Arc<Server> {
        let server_callback = Arc::new(RwLock::new(None));
        let inner_server = server_callback.clone();
        let mut inner_server = inner_server.write().unwrap();
        let server = Arc::new(Server::new(
            protocol::SUPPORTED_PROTOCOLS[0],
            vec![],
            protocol::UUID::default(),
            resources,
            Arc::new(RwLock::new(None)),
            Self::spawn_light_updater(server_callback.clone())
        ));
        inner_server.replace(server.clone());
        println!("instantiated server!");
        let mut rng = rand::thread_rng();
        // TODO: Fix the following startup bottleneck!
        // (-7 * 16..7 * 16).into_par_iter().sum();

        for x in (-7 * 16)..(7 * 16) { // TODO: Use par iters
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
        println!("built server!");
        server.clone()
    }

    fn new(
        protocol_version: i32,
        forge_mods: Vec<forge::ForgeMod>,
        uuid: protocol::UUID,
        resources: Arc<RwLock<resources::Manager>>,
        conn: Arc<RwLock<Option<protocol::Conn>>>,
        light_updater: mpsc::Sender<bool>,
    ) -> Server {
        let mut entities = ecs::Manager::new();
        entity::add_systems(&mut entities);

        let world_entity = entities.get_world();
        let game_info = entities.get_key();
        entities.add_component(world_entity, game_info, entity::GameInfo::new());

        let version = resources.read().unwrap().version();
        Server {
            uuid,
            conn,
            protocol_version,
            forge_mods,
            disconnect_data: Arc::new(RwLock::new(DisconnectData::default())),

            world: Arc::new(world::World::new(protocol_version)),
            world_data: Arc::new(RwLock::new(WorldData::default())),
            version: RwLock::new(Some(version)),
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
            entity_map: Arc::new(RwLock::new(HashMap::with_hasher(BuildHasherDefault::default()))),
            players: Arc::new(RwLock::new(HashMap::with_hasher(BuildHasherDefault::default()))),

            tick_timer: RwLock::from(0.0),
            entity_tick_timer: RwLock::from(0.0),
            received_chat_at: Arc::new(RwLock::new(None)),
            sun_model: RwLock::new(None),

            target_info: Arc::new(RwLock::new(target::Info::new())),
            light_updates: Mutex::from(light_updater)
        }
    }

    pub fn disconnect(&self, reason: Option<format::Component>) {
        self.conn.clone().write().unwrap().take();
        self.disconnect_data.clone().write().unwrap().disconnect_reason = reason;
        if let Some(player) = self.player.clone().write().unwrap().take() {
            self.entities.clone().write().unwrap().remove_entity(player);
        }
        self.disconnect_data.clone().write().unwrap().just_disconnected = true;
    }

    pub fn is_connected(&self) -> bool {
        self.conn.clone().read().unwrap().is_some()
    }

    pub fn tick(&self, renderer: &mut render::Renderer, delta: f64) {
        // let now = Instant::now();
        let version = self.resources.read().unwrap().version();
        if version != self.version.read().unwrap().as_ref().unwrap().clone() {
            self.version.write().unwrap().replace(version);
            self.world.clone().flag_dirty_all();
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff1 took {}", diff.as_millis());*/
        // TODO: Check if the world type actually needs a sun
        if self.sun_model.read().unwrap().is_none() {
            self.sun_model.write().unwrap().replace(sun::SunModel::new(renderer));
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff2 took {}", diff.as_millis());*/

        // Copy to camera
        if let Some(player) = *self.player.clone().read().unwrap() {
            let position = self.entities.clone().read().unwrap().get_component(player, self.position).unwrap();
            let rotation = self.entities.clone().read().unwrap().get_component(player, self.rotation).unwrap();
            renderer.camera.pos =
                cgmath::Point3::from_vec(position.position + cgmath::Vector3::new(0.0, 1.62, 0.0));
            renderer.camera.yaw = rotation.yaw;
            renderer.camera.pitch = rotation.pitch;
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff3 took {}", diff.as_millis());*/
        // println!("entity_tick!");
        self.entity_tick(renderer, delta);
        /*let diff = Instant::now().duration_since(now);
        println!("Diff4 took {}", diff.as_millis());*/

        *self.tick_timer.write().unwrap() += delta;
        while self.tick_timer.read().unwrap().clone() >= 3.0 && self.is_connected() {
            self.minecraft_tick();
            *self.tick_timer.write().unwrap() -= 3.0;
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff5 took {}", diff.as_millis());*/

        self.update_time(renderer, delta);
        /*let diff = Instant::now().duration_since(now);
        println!("Diff6 took {}", diff.as_millis());*/
        if let Some(sun_model) = self.sun_model.write().unwrap().as_mut() {
            sun_model.tick(renderer, self.world_data.clone().read().unwrap().world_time, self.world_data.clone().read().unwrap().world_age);
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff7 took {}", diff.as_millis());*/
        let world = self.world.clone();
        world.tick(&mut self.entities.clone().write().unwrap());
        // if !world.light_updates.clone().read().unwrap().is_empty() { // TODO: Check if removing this is okay!
            self.light_updates.lock().unwrap().send(true);
        // }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff8 took {}", diff.as_millis());*/

        if self.player.clone().read().unwrap().is_some() {
            let world = self.world.clone();
            if let Some((pos, bl, _, _)) = target::trace_ray(
                &world/*&self.world*/,
                4.0,
                renderer.camera.pos.to_vec(),
                renderer.view_vector.cast().unwrap(),
                target::test_block,
            ) {
                self.target_info.clone().write().unwrap().update(renderer, pos, bl);
            } else {
                self.target_info.clone().write().unwrap().clear(renderer);
            }
        } else {
            self.target_info.clone().write().unwrap().clear(renderer);
        }
        /*let diff = Instant::now().duration_since(now);
        println!("Diff9 took {}", diff.as_millis());*/
    }
    // diff 8 is to be investigated!
    // When in main menu, diff 8 is the most influencial by far!


    fn entity_tick(&self, renderer: &mut render::Renderer, delta: f64) {
        let world_entity = self.entities.clone().read().unwrap().get_world();
        // Update the game's state for entities to read
        self.entities
            .clone().write().unwrap()
            .get_component_mut(world_entity, self.game_info)
            .unwrap()
            .delta = delta;

        // Packets modify entities so need to handled here
        // TODO: Find a better solution cuz this kills performance entirely!
        /*if let Some(rx) = self.read_queue.take() {
            while let Ok(pck) = rx.try_recv() {
                match pck {
                    Ok(pck) => handle_packet! {
                        self pck {
                            KeepAliveClientbound_i64 => on_keep_alive_i64,
                            KeepAliveClientbound_VarInt => on_keep_alive_varint,
                            KeepAliveClientbound_i32 => on_keep_alive_i32,
                            PluginMessageClientbound_i16 => on_plugin_message_clientbound_i16,
                            PluginMessageClientbound => on_plugin_message_clientbound_1,
                            // JoinGame_WorldNames_IsHard => on_game_join_worldnames_ishard,
                            // JoinGame_WorldNames => on_game_join_worldnames,
                            // JoinGame_HashedSeed_Respawn => on_game_join_hashedseed_respawn,
                            // JoinGame_i32_ViewDistance => on_game_join_i32_viewdistance,
                            // JoinGame_i32 => on_game_join_i32,
                            // JoinGame_i8 => on_game_join_i8,
                            // JoinGame_i8_NoDebug => on_game_join_i8_nodebug,
                            // Respawn_Gamemode => on_respawn_gamemode,
                            // Respawn_HashedSeed => on_respawn_hashedseed,
                            // Respawn_WorldName => on_respawn_worldname,
                            // Respawn_NBT => on_respawn_nbt,
                            // ChunkData_Biomes3D_VarInt => on_chunk_data_biomes3d_varint,
                            // ChunkData_Biomes3D_bool => on_chunk_data_biomes3d_bool,
                            // ChunkData => on_chunk_data,
                            // ChunkData_Biomes3D => on_chunk_data_biomes3d, // This causes things not to get rendered on unicat!
                            // ChunkData_HeightMap => on_chunk_data_heightmap,
                            // ChunkData_NoEntities => on_chunk_data_no_entities,
                            // ChunkData_NoEntities_u16 => on_chunk_data_no_entities_u16,
                            // ChunkData_17 => on_chunk_data_17,
                            // ChunkDataBulk => on_chunk_data_bulk,
                            // ChunkDataBulk_17 => on_chunk_data_bulk_17,
                            // ChunkUnload => on_chunk_unload,
                            // BlockChange_VarInt => on_block_change_varint,
                            // BlockChange_u8 => on_block_change_u8,
                            // MultiBlockChange_Packed => on_multi_block_change_packed,
                            // MultiBlockChange_VarInt => on_multi_block_change_varint,
                            // MultiBlockChange_u16 => on_multi_block_change_u16,
                            // TeleportPlayer_WithConfirm => on_teleport_player_withconfirm,
                            // TeleportPlayer_NoConfirm => on_teleport_player_noconfirm,
                            // TeleportPlayer_OnGround => on_teleport_player_onground,
                            TimeUpdate => on_time_update,
                            ChangeGameState => on_game_state_change,
                            // UpdateBlockEntity => on_block_entity_update,
                            // UpdateBlockEntity_Data => on_block_entity_update_data,
                            // UpdateSign => on_sign_update,
                            // UpdateSign_u16 => on_sign_update_u16,
                            // PlayerInfo => on_player_info,
                            PlayerInfo_String => on_player_info_string,
                            ServerMessage_NoPosition => on_servermessage_noposition,
                            ServerMessage_Position => on_servermessage_position,
                            ServerMessage_Sender => on_servermessage_sender,
                            Disconnect => on_disconnect,
                            // Entities
                            // EntityDestroy => on_entity_destroy,
                            // EntityDestroy_u8 => on_entity_destroy_u8,
                            // SpawnPlayer_f64_NoMeta => on_player_spawn_f64_nometa,
                            // SpawnPlayer_f64 => on_player_spawn_f64,
                            // SpawnPlayer_i32 => on_player_spawn_i32,
                            // SpawnPlayer_i32_HeldItem => on_player_spawn_i32_helditem,
                            // SpawnPlayer_i32_HeldItem_String => on_player_spawn_i32_helditem_string,
                            // EntityTeleport_f64 => on_entity_teleport_f64,
                            // EntityTeleport_i32 => on_entity_teleport_i32,
                            // EntityTeleport_i32_i32_NoGround => on_entity_teleport_i32_i32_noground,
                            // EntityMove_i16 => on_entity_move_i16,
                            // EntityMove_i8 => on_entity_move_i8,
                            // EntityMove_i8_i32_NoGround => on_entity_move_i8_i32_noground,
                            // EntityLook_VarInt => on_entity_look_varint,
                            // EntityLook_i32_NoGround => on_entity_look_i32_noground,
                            // EntityLookAndMove_i16 => on_entity_look_and_move_i16,
                            // EntityLookAndMove_i8 => on_entity_look_and_move_i8,
                            // EntityLookAndMove_i8_i32_NoGround => on_entity_look_and_move_i8_i32_noground,
                        }
                    },
                    Err(err) => panic!("Err: {:?}", err), // TODO: Fix this: thread 'main' panicked at 'Err: IOError(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" })', src/server/mod.rs:679:33
                }
                // Disconnected
                if self.conn.is_none() {
                    break;
                }
            }

            if self.conn.is_some() {
                self.read_queue.lock().unwrap().replace(rx);
                if self.write_queue.lock().unwrap().is_some() {
                    let mut tmp = self.conn.take().unwrap();
                    tmp.send = self.write_queue.take();
                    self.conn = Some(tmp);
                }
            }
        }*/

        if self.is_connected() || self.disconnect_data.clone().read().unwrap().just_disconnected {
            // Allow an extra tick when disconnected to clean up
            self.disconnect_data.clone().write().unwrap().just_disconnected = false;
            *self.entity_tick_timer.write().unwrap() += delta;
            while self.entity_tick_timer.read().unwrap().clone() >= 3.0 {
                let world = self.world.clone();
                self.entities.clone().write().unwrap().tick(&world/*&mut self.world*/, renderer);
                *self.entity_tick_timer.write().unwrap() -= 3.0;
            }
            let world = self.world.clone();
            self.entities
                .clone().write().unwrap()
                .render_tick(&world/*&mut self.world*/, renderer);
        }
    }

    pub fn remove(&mut self, renderer: &mut render::Renderer) {
        let world = self.world.clone();
        self.entities
            .clone().write().unwrap()
            .remove_all_entities(&world/*&mut self.world*/, renderer);
        if let Some(sun_model) = self.sun_model.write().unwrap().as_mut() {
            sun_model.remove(renderer);
        }
        self.target_info.clone().write().unwrap().clear(renderer);
    }

    fn update_time(&self, renderer: &mut render::Renderer, delta: f64) {
        if self.world_data.clone().read().unwrap().tick_time {
            self.world_data.clone().write().unwrap().world_time_target += delta / 3.0;
            let time = self.world_data.clone().read().unwrap().world_time_target;
            self.world_data.clone().write().unwrap().world_time_target = (24000.0 + time) % 24000.0;
            let mut diff = self.world_data.clone().read().unwrap().world_time_target - self.world_data.clone().read().unwrap().world_time;
            if diff < -12000.0 {
                diff += 24000.0
            } else if diff > 12000.0 {
                diff -= 24000.0
            }
            self.world_data.clone().write().unwrap().world_time += diff * (1.5 / 60.0) * delta;
            let time = self.world_data.clone().read().unwrap().world_time;
            self.world_data.clone().write().unwrap().world_time = (24000.0 + time) % 24000.0;
        } else {
            let time = self.world_data.clone().read().unwrap().world_time_target;
            self.world_data.clone().write().unwrap().world_time = time;
        }
        renderer.sky_offset = self.calculate_sky_offset();
    }

    fn calculate_sky_offset(&self) -> f32 {
        use std::f32::consts::PI;
        let mut offset = ((1.0 + self.world_data.clone().read().unwrap().world_time as f32) / 24000.0) - 0.25;
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
        if let Some(player) = *self.player.clone().write().unwrap() {
            let movement = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, self.player_movement)
                .unwrap();
            let on_ground = self
                .entities
                .clone().read().unwrap()
                .get_component(player, self.gravity)
                .map_or(false, |v| v.on_ground);
            let position = self
                .entities
                .clone().read().unwrap()
                .get_component(player, self.target_position)
                .unwrap();
            let rotation = self.entities
                .clone().read().unwrap()
                .get_component(player, self.rotation).unwrap();

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
    }

    pub fn key_press(&self, down: bool, key: Actionkey) {
        if let Some(player) = *self.player.clone().write().unwrap() {
            if let Some(movement) = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, self.player_movement)
            {
                println!("pressing movement key!");
                movement.pressed_keys.insert(key, down);
            }
        }
    }

    pub fn on_right_click(&self, renderer: &mut render::Renderer) {
        use crate::shared::Direction;
        if self.player.clone().read().unwrap().is_some() {
            let world = self.world.clone();
            if let Some((pos, _, face, at)) = target::trace_ray(
                &world/*&self.world*/,
                4.0,
                renderer.camera.pos.to_vec(),
                renderer.view_vector.cast().unwrap(),
                target::test_block,
            ) {
                if self.protocol_version >= 477 {
                    self.write_packet(
                        packet::play::serverbound::PlayerBlockPlacement_insideblock {
                            location: pos,
                            face: protocol::VarInt(match face {
                                Direction::Down => 0,
                                Direction::Up => 1,
                                Direction::North => 2,
                                Direction::South => 3,
                                Direction::West => 4,
                                Direction::East => 5,
                                _ => unreachable!(),
                            }),
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
                        face: protocol::VarInt(match face {
                            Direction::Down => 0,
                            Direction::Up => 1,
                            Direction::North => 2,
                            Direction::South => 3,
                            Direction::West => 4,
                            Direction::East => 5,
                            _ => unreachable!(),
                        }),
                        hand: protocol::VarInt(0),
                        cursor_x: at.x as f32,
                        cursor_y: at.y as f32,
                        cursor_z: at.z as f32,
                    });
                } else if self.protocol_version >= 49 {
                    self.write_packet(packet::play::serverbound::PlayerBlockPlacement_u8 {
                        location: pos,
                        face: protocol::VarInt(match face {
                            Direction::Down => 0,
                            Direction::Up => 1,
                            Direction::North => 2,
                            Direction::South => 3,
                            Direction::West => 4,
                            Direction::East => 5,
                            _ => unreachable!(),
                        }),
                        hand: protocol::VarInt(0),
                        cursor_x: (at.x * 16.0) as u8,
                        cursor_y: (at.y * 16.0) as u8,
                        cursor_z: (at.z * 16.0) as u8,
                    });
                } else if self.protocol_version >= 47 {
                    self.write_packet(packet::play::serverbound::PlayerBlockPlacement_u8_Item {
                        location: pos,
                        face: match face {
                            Direction::Down => 0,
                            Direction::Up => 1,
                            Direction::North => 2,
                            Direction::South => 3,
                            Direction::West => 4,
                            Direction::East => 5,
                            _ => unreachable!(),
                        },
                        hand: None,
                        cursor_x: (at.x * 16.0) as u8,
                        cursor_y: (at.y * 16.0) as u8,
                        cursor_z: (at.z * 16.0) as u8,
                    });
                } else {
                    self.write_packet(
                        packet::play::serverbound::PlayerBlockPlacement_u8_Item_u8y {
                            x: pos.x,
                            y: pos.y as u8,
                            z: pos.x,
                            face: match face {
                                Direction::Down => 0,
                                Direction::Up => 1,
                                Direction::North => 2,
                                Direction::South => 3,
                                Direction::West => 4,
                                Direction::East => 5,
                                _ => unreachable!(),
                            },
                            hand: None,
                            cursor_x: (at.x * 16.0) as u8,
                            cursor_y: (at.y * 16.0) as u8,
                            cursor_z: (at.z * 16.0) as u8,
                        },
                    );
                }
            }
        }
    }

    pub fn write_packet<T: protocol::PacketType>(&self, p: T) {
        let _ = self.conn.clone().write().unwrap().as_mut().unwrap().write_packet(p); // TODO handle errors
    }

    fn on_keep_alive_i64(
        &mut self,
        keep_alive: packet::play::clientbound::KeepAliveClientbound_i64,
    ) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_i64 {
            id: keep_alive.id,
        });
    }

    fn on_keep_alive_varint(
        &mut self,
        keep_alive: packet::play::clientbound::KeepAliveClientbound_VarInt,
    ) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_VarInt {
            id: keep_alive.id,
        });
    }

    fn on_keep_alive_i32(
        &mut self,
        keep_alive: packet::play::clientbound::KeepAliveClientbound_i32,
    ) {
        self.write_packet(packet::play::serverbound::KeepAliveServerbound_i32 {
            id: keep_alive.id,
        });
    }

    fn on_plugin_message_clientbound_i16(
        &mut self,
        msg: packet::play::clientbound::PluginMessageClientbound_i16,
    ) {
        self.on_plugin_message_clientbound(&msg.channel, msg.data.data.as_slice())
    }

    fn on_plugin_message_clientbound_1(
        &mut self,
        msg: packet::play::clientbound::PluginMessageClientbound,
    ) {
        self.on_plugin_message_clientbound(&msg.channel, &msg.data)
    }

    fn on_plugin_message_clientbound(&mut self, channel: &str, data: &[u8]) {
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
                //debug!("FML|HS msg={:?}", msg);

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
                                self.world.clone()
                                    .modded_block_ids.clone().write().unwrap()
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
                                self.world.clone().modded_block_ids.clone().write().unwrap().insert(m.id.0 as usize, m.name);
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

    fn on_plugin_message_clientbound_i16_glob(
        server: Arc<Server>,
        msg: packet::play::clientbound::PluginMessageClientbound_i16,
    ) {
        Server::on_plugin_message_clientbound_glob(server, &msg.channel, msg.data.data.as_slice())
    }

    fn on_plugin_message_clientbound_1_glob(
        server: Arc<Server>,
        msg: packet::play::clientbound::PluginMessageClientbound,
    ) {
        Server::on_plugin_message_clientbound_glob(server, &msg.channel, &msg.data)
    }

    fn on_plugin_message_clientbound_glob(server: Arc<Server>, channel: &str, data: &[u8]) {
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
                //debug!("FML|HS msg={:?}", msg);

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

                        server.write_plugin_message("REGISTER", b"FML|HS\0FML\0FML|MP\0FML\0FORGE");
                        server.write_fmlhs_plugin_message(&ClientHello {
                            fml_protocol_version,
                        });
                        // Send stashed mods list received from ping packet, client matching server
                        let mods = crate::protocol::LenPrefixed::<
                            crate::protocol::VarInt,
                            forge::ForgeMod,
                        >::new(server.forge_mods.clone());
                        server.write_fmlhs_plugin_message(&ModList { mods });
                    }
                    ModList { mods } => {
                        debug!("Received FML|HS ModList: {:?}", mods);

                        server.write_fmlhs_plugin_message(&HandshakeAck {
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
                                server.world.clone()
                                    .modded_block_ids.clone().write().unwrap()
                                    .insert(m.id.0 as usize, name.to_string());
                            }
                        }
                        server.write_fmlhs_plugin_message(&HandshakeAck {
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
                                server.world.clone().modded_block_ids.clone().write().unwrap().insert(m.id.0 as usize, m.name);
                            }
                        }
                        if !has_more {
                            server.write_fmlhs_plugin_message(&HandshakeAck {
                                phase: WaitingServerComplete,
                            });
                        }
                    }
                    HandshakeAck { phase } => match phase {
                        WaitingCAck => {
                            server.write_fmlhs_plugin_message(&HandshakeAck {
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

    // TODO: remove wrappers and directly call on Conn
    fn write_fmlhs_plugin_message(&self, msg: &forge::FmlHs) {
        let _ = self.conn.clone().write().unwrap().as_mut().unwrap().write_fmlhs_plugin_message(msg); // TODO handle errors
    }

    fn write_plugin_message(&self, channel: &str, data: &[u8]) {
        let _ = self
            .conn
            .clone().write().unwrap().as_mut().unwrap()
            .write_plugin_message(channel, data); // TODO handle errors
    }

    fn on_game_join_worldnames_ishard(
        &mut self,
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
        let gamemode = Gamemode::from_int((gamemode & 0x7) as i32);
        let player = entity::player::create_local(&mut self.entities.clone().write().unwrap());
        if let Some(info) = self.players.clone().read().unwrap().get(&self.uuid) {
            let model = self
                .entities
                .clone().write().unwrap()
                .get_component_mut_direct::<entity::player::PlayerModel>(player)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        *self
            .entities
            .clone().write().unwrap()
            .get_component_mut(player, self.gamemode)
            .unwrap() = gamemode;
        // TODO: Temp
        self.entities
            .clone().write().unwrap()
            .get_component_mut(player, self.player_movement)
            .unwrap()
            .flying = gamemode.can_fly();

        self.entity_map.clone().write().unwrap().insert(entity_id, player);
        self.player.clone().write().unwrap().replace(player);

        // Let the server know who we are
        let brand = plugin_messages::Brand {
            brand: "leafish".into(),
        };
        // TODO: refactor with write_plugin_message
        // TODO: Try sending ClientSettings right here! (leafishish)
        if self.protocol_version >= 47 {
            self.write_packet(brand.into_message());
        } else {
            self.write_packet(brand.into_message17());
        }
    }

    fn on_game_join_glob(
        players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,
        internal_player: Arc<RwLock<Option<ecs::Entity>>>,
        entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
        player_movement: Key<PlayerMovement>,
        entities: Arc<RwLock<Manager>>,
        gamemode_key: Key<Gamemode>,
        uuid: &protocol::UUID,
        protocol_version: i32,
        read: &mut protocol::Conn,
        gamemode: u8, entity_id: i32) {
        let gamemode = Gamemode::from_int((gamemode & 0x7) as i32);
        let player = entity::player::create_local(&mut entities.clone().write().unwrap());
        if let Some(info) = players.clone().read().unwrap().get(uuid) {
            let model = entities
                .clone().write().unwrap()
                .get_component_mut_direct::<entity::player::PlayerModel>(player)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        *entities
            .clone().write().unwrap()
            .get_component_mut(player, gamemode_key)
            .unwrap() = gamemode;
        // TODO: Temp
        entities
            .clone().write().unwrap()
            .get_component_mut(player, player_movement)
            .unwrap()
            .flying = gamemode.can_fly();

        entity_map.clone().write().unwrap().insert(entity_id, player);
        internal_player.clone().write().unwrap().replace(player);

        // Let the server know who we are
        let brand = plugin_messages::Brand {
            brand: "leafish".into(),
        };
        // TODO: refactor with write_plugin_message
        // TODO: Try sending ClientSettings right here! (leafishish)
        if protocol_version >= 47 {
            read.write_packet(brand.into_message());
        } else {
            read.write_packet(brand.into_message17());
        }
    }

    /*
    fn on_respawn_hashedseed(&mut self, respawn: packet::play::clientbound::Respawn_HashedSeed) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_gamemode(&mut self, respawn: packet::play::clientbound::Respawn_Gamemode) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_worldname(&mut self, respawn: packet::play::clientbound::Respawn_WorldName) {
        self.respawn(respawn.gamemode)
    }

    fn on_respawn_nbt(&mut self, respawn: packet::play::clientbound::Respawn_NBT) {
        self.respawn(respawn.gamemode)
    }

    fn respawn(&mut self, gamemode_u8: u8) {
        self.world = Arc::new(RwLock::new(Some(world::World::new(self.protocol_version))));
        let gamemode = Gamemode::from_int((gamemode_u8 & 0x7) as i32);

        if let Some(player) = *self.player.clone().write().unwrap() {
            *self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, *self.gamemode.clone())
                .unwrap() = gamemode;
            // TODO: Temp
            self.entities
                .clone().write().unwrap()
                .get_component_mut(player, *self.player_movement.clone())
                .unwrap()
                .flying = gamemode.can_fly();
        }
    }*/

    fn respawn_glob(
        world: Arc<World>,
        internal_player: Arc<RwLock<Option<ecs::Entity>>>,
        player_movement: Key<PlayerMovement>,
        entities: Arc<RwLock<Manager>>,
        gamemode_key: Key<Gamemode>,
        protocol_version: i32,
        gamemode_u8: u8) {
        // world.clone().replace(world::World::new(protocol_version));
        world.clone().reset(protocol_version);
        let gamemode = Gamemode::from_int((gamemode_u8 & 0x7) as i32);

        if let Some(player) = *internal_player.clone().write().unwrap() {
            *entities
                .clone().write().unwrap()
                .get_component_mut(player, gamemode_key)
                .unwrap() = gamemode;
            // TODO: Temp
            entities
                .clone().write().unwrap()
                .get_component_mut(player, player_movement)
                .unwrap()
                .flying = gamemode.can_fly();
        }
    }

    fn on_disconnect(&mut self, disconnect: packet::play::clientbound::Disconnect) {
        self.disconnect(Some(disconnect.reason));
    }

    fn on_time_update(&mut self, time_update: packet::play::clientbound::TimeUpdate) {
        self.world_data.clone().write().unwrap().world_age = time_update.time_of_day;
        self.world_data.clone().write().unwrap().world_time_target = (time_update.time_of_day % 24000) as f64;
        if self.world_data.clone().read().unwrap().world_time_target < 0.0 {
            self.world_data.clone().write().unwrap().world_time_target = -self.world_data.clone().read().unwrap().world_time_target;
            self.world_data.clone().write().unwrap().tick_time = false;
        } else {
            self.world_data.clone().write().unwrap().tick_time = true;
        }
    }

    fn on_game_state_change(&mut self, game_state: packet::play::clientbound::ChangeGameState) {
        println!("game state change!");
        if game_state.reason == 3 {
            if let Some(player) = *self.player.write().unwrap() {
                let gamemode = Gamemode::from_int(game_state.value as i32);
                *self
                    .entities
                    .clone().write().unwrap()
                    .get_component_mut(player, self.gamemode)
                    .unwrap() = gamemode;
                // TODO: Temp
                self.entities
                    .clone().write().unwrap()
                    .get_component_mut(player, self.player_movement)
                    .unwrap()
                    .flying = gamemode.can_fly();
            }
        }
    }

    /*
    fn on_entity_destroy(&mut self, entity_destroy: packet::play::clientbound::EntityDestroy) {
        for id in entity_destroy.entity_ids.data {
            if let Some(entity) = self.entity_map.clone().write().unwrap().remove(&id.0) {
                self.entities.clone().write().unwrap().remove_entity(entity);
            }
        }
    }*/

    /*
    fn on_entity_destroy_u8(
        &mut self,
        entity_destroy: packet::play::clientbound::EntityDestroy_u8,
    ) {
        for id in entity_destroy.entity_ids.data {
            if let Some(entity) = self.entity_map.clone().write().unwrap().remove(&id) {
                self.entities.clone().write().unwrap().remove_entity(entity);
            }
        }
    }*/

    /*
    fn on_entity_teleport_f64(
        &mut self,
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
    }*/

    /*
    fn on_entity_teleport_i32(
        &mut self,
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
    }*/

    /*
    fn on_entity_teleport_i32_i32_noground(
        &mut self,
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
    }*/

    fn on_entity_teleport(
        &mut self,
        entity_id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        _on_ground: bool,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().unwrap().get(&entity_id) {
            let target_position = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            let target_rotation = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
            target_rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            target_rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_teleport_glob(
        entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
        target_position: Key<TargetPosition>,
        target_rotation: Key<TargetRotation>,
        entities: Arc<RwLock<Manager>>,
        entity_id: i32,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        _on_ground: bool,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = entity_map.clone().read().unwrap().get(&entity_id) {
            let target_position = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_position)
                .unwrap();
            let target_rotation = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_rotation)
                .unwrap();
            target_position.position.x = x;
            target_position.position.y = y;
            target_position.position.z = z;
            target_rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            target_rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    /*
    fn on_entity_move_i16(&mut self, m: packet::play::clientbound::EntityMove_i16) {
        self.on_entity_move(
            m.entity_id.0,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }*/

    /*
    fn on_entity_move_i8(&mut self, m: packet::play::clientbound::EntityMove_i8) {
        self.on_entity_move(
            m.entity_id.0,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }*/

    /*
    fn on_entity_move_i8_i32_noground(
        &mut self,
        m: packet::play::clientbound::EntityMove_i8_i32_NoGround,
    ) {
        self.on_entity_move(
            m.entity_id,
            f64::from(m.delta_x),
            f64::from(m.delta_y),
            f64::from(m.delta_z),
        )
    }*/

    fn on_entity_move(&mut self, entity_id: i32, delta_x: f64, delta_y: f64, delta_z: f64) {
        if let Some(entity) = self.entity_map.clone().read().unwrap().get(&entity_id) {
            let position = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
        }
    }

    fn on_entity_move_glob(entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
                           target_position: Key<TargetPosition>,
                           entities: Arc<RwLock<Manager>>,
                           entity_id: i32, delta_x: f64, delta_y: f64, delta_z: f64) {
        if let Some(entity) = entity_map.clone().read().unwrap().get(&entity_id) {
            let position = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_position)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
        }
    }

    fn on_entity_look(&mut self, entity_id: i32, yaw: f64, pitch: f64) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().unwrap().get(&entity_id) {
            let rotation = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_look_glob(entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
                           target_rotation: Key<TargetRotation>,
                           entities: Arc<RwLock<Manager>>,
                           entity_id: i32, yaw: f64, pitch: f64) {
        use std::f64::consts::PI;
        if let Some(entity) = entity_map.clone().read().unwrap().get(&entity_id) {
            let rotation = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_rotation)
                .unwrap();
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    /*
    fn on_entity_look_varint(&mut self, look: packet::play::clientbound::EntityLook_VarInt) {
        self.on_entity_look(look.entity_id.0, look.yaw as f64, look.pitch as f64)
    }*/

    /*
    fn on_entity_look_i32_noground(
        &mut self,
        look: packet::play::clientbound::EntityLook_i32_NoGround,
    ) {
        self.on_entity_look(look.entity_id, look.yaw as f64, look.pitch as f64)
    }*/

    /*
    fn on_entity_look_and_move_i16(
        &mut self,
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
    }*/

    /*
    fn on_entity_look_and_move_i8(
        &mut self,
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
        &mut self,
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
    }*/

    fn on_entity_look_and_move(
        &mut self,
        entity_id: i32,
        delta_x: f64,
        delta_y: f64,
        delta_z: f64,
        yaw: f64,
        pitch: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().read().unwrap().get(&entity_id) {
            let position = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_position)
                .unwrap();
            let rotation = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(*entity, self.target_rotation)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    fn on_entity_look_and_move_glob(
        target_position: Key<TargetPosition>,
        target_rotation: Key<TargetRotation>,
        entities: Arc<RwLock<Manager>>,
        entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
        entity_id: i32,
        delta_x: f64,
        delta_y: f64,
        delta_z: f64,
        yaw: f64,
        pitch: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = entity_map.clone().read().unwrap().get(&entity_id) {
            let position = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_position)
                .unwrap();
            let rotation = entities
                .clone().write().unwrap()
                .get_component_mut(*entity, target_rotation)
                .unwrap();
            position.position.x += delta_x;
            position.position.y += delta_y;
            position.position.z += delta_z;
            rotation.yaw = -(yaw / 256.0) * PI * 2.0;
            rotation.pitch = -(pitch / 256.0) * PI * 2.0;
        }
    }

    /*
    fn on_player_spawn_f64_nometa(
        &mut self,
        spawn: packet::play::clientbound::SpawnPlayer_f64_NoMeta,
    ) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            spawn.x,
            spawn.y,
            spawn.z,
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }*/

    /*
    fn on_player_spawn_f64(&mut self, spawn: packet::play::clientbound::SpawnPlayer_f64) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            spawn.x,
            spawn.y,
            spawn.z,
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }*/

    /*
    fn on_player_spawn_i32(&mut self, spawn: packet::play::clientbound::SpawnPlayer_i32) {
        self.on_player_spawn(
            spawn.entity_id.0,
            spawn.uuid,
            f64::from(spawn.x),
            f64::from(spawn.y),
            f64::from(spawn.z),
            spawn.yaw as f64,
            spawn.pitch as f64,
        )
    }*/

    /*
    fn on_player_spawn_i32_helditem(
        &mut self,
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
        &mut self,
        spawn: packet::play::clientbound::SpawnPlayer_i32_HeldItem_String,
    ) {
        // 1.7.10: populate the player list here, since we only now know the UUID
        let uuid = protocol::UUID::from_str(&spawn.uuid).unwrap();
        self.players.clone().write().unwrap().entry(uuid.clone()).or_insert(PlayerInfo {
            name: spawn.name.clone(),
            uuid,
            skin_url: None,

            display_name: None,
            ping: 0, // TODO: don't overwrite from PlayerInfo_String
            gamemode: Gamemode::from_int(0),
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
    }*/

    fn on_player_spawn(
        &mut self,
        entity_id: i32,
        uuid: protocol::UUID,
        x: f64,
        y: f64,
        z: f64,
        pitch: f64,
        yaw: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = self.entity_map.clone().write().unwrap().remove(&entity_id) {
            self.entities.clone().write().unwrap().remove_entity(entity);
        }
        let entity = entity::player::create_remote(
            &mut self.entities.clone().write().unwrap(),
            self.players.clone().read().unwrap().get(&uuid).map_or("MISSING", |v| &v.name),
        );
        let position = self
            .entities
            .clone().write().unwrap()
            .get_component_mut(entity, self.position)
            .unwrap();
        let target_position = self
            .entities
            .clone().write().unwrap()
            .get_component_mut(entity, self.target_position)
            .unwrap();
        let rotation = self
            .entities
            .clone().write().unwrap()
            .get_component_mut(entity, self.rotation)
            .unwrap();
        let target_rotation = self
            .entities
            .clone().write().unwrap()
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
        if let Some(info) = self.players.clone().read().unwrap().get(&uuid) {
            let model = self
                .entities
                .clone().write().unwrap()
                .get_component_mut_direct::<entity::player::PlayerModel>(entity)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        self.entity_map.clone().write().unwrap().insert(entity_id, entity);
    }

    fn on_player_spawn_glob(
        target_position_key: Key<TargetPosition>,
        target_rotation_key: Key<TargetRotation>,
        entities: Arc<RwLock<Manager>>,
        entity_map: Arc<RwLock<HashMap<i32, ecs::Entity, BuildHasherDefault<FNVHash>>>>,
        players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,
        position_key: Key<entity::Position>,
        rotation_key: Key<entity::Rotation>,
        entity_id: i32,
        uuid: protocol::UUID,
        x: f64,
        y: f64,
        z: f64,
        pitch: f64,
        yaw: f64,
    ) {
        use std::f64::consts::PI;
        if let Some(entity) = entity_map.clone().write().unwrap().remove(&entity_id) {
            entities.clone().write().unwrap().remove_entity(entity);
        }
        let entity = entity::player::create_remote(
            &mut entities.clone().write().unwrap(),
            players.clone().read().unwrap().get(&uuid).map_or("MISSING", |v| &v.name),
        );
        let position = entities
            .clone().write().unwrap()
            .get_component_mut(entity, position_key)
            .unwrap();
        let target_position = entities
            .clone().write().unwrap()
            .get_component_mut(entity, target_position_key)
            .unwrap();
        let rotation = entities
            .clone().write().unwrap()
            .get_component_mut(entity, rotation_key)
            .unwrap();
        let target_rotation = entities
            .clone().write().unwrap()
            .get_component_mut(entity, target_rotation_key)
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
        if let Some(info) = players.clone().read().unwrap().get(&uuid) {
            let model = entities
                .clone().write().unwrap()
                .get_component_mut_direct::<entity::player::PlayerModel>(entity)
                .unwrap();
            model.set_skin(info.skin_url.clone());
        }
        entity_map.clone().write().unwrap().insert(entity_id, entity);
    }


    /*
    fn on_teleport_player_withconfirm(
        &mut self,
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
        &mut self,
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
    }*/

    /*
    fn on_teleport_player_onground(
        &mut self,
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
    }*/

    /*
    fn on_teleport_player(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        flags: u8,
        teleport_id: Option<protocol::VarInt>,
    ) {
        use std::f64::consts::PI;
        if let Some(player) = *self.player.clone().write().unwrap() {
            let position = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, *self.target_position.clone())
                .unwrap();
            let rotation = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, *self.rotation.clone())
                .unwrap();
            let velocity = self
                .entities
                .clone().write().unwrap()
                .get_component_mut(player, *self.velocity.clone())
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
    }*/

    fn on_teleport_player_glob(
        internal_player: Arc<RwLock<Option<ecs::Entity>>>,
        target_position: Key<TargetPosition>,
        entities: Arc<RwLock<Manager>>,
        velocity_key: Key<entity::Velocity>,
        rotation_key: Key<entity::Rotation>,
        read: &mut protocol::Conn,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
        flags: u8,
        teleport_id: Option<protocol::VarInt>,
    ) {
        use std::f64::consts::PI;
        if let Some(player) = *internal_player.clone().write().unwrap() {
            let position = entities
                .clone().write().unwrap()
                .get_component_mut(player, target_position)
                .unwrap();
            let rotation = entities
                .clone().write().unwrap()
                .get_component_mut(player, rotation_key)
                .unwrap();
            let velocity = entities
                .clone().write().unwrap()
                .get_component_mut(player, velocity_key)
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
                read.write_packet(packet::play::serverbound::TeleportConfirm { teleport_id });
            }
        }
    }

    // TODO: Move to world!
    fn on_block_entity_update(
        &mut self,
        block_update: packet::play::clientbound::UpdateBlockEntity,
    ) {
        match block_update.nbt {
            None => {
                // NBT is null, so we need to remove the block entity
                self.world.clone()
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

    fn on_block_entity_update_glob(
        world: Arc<World>,
        block_update: packet::play::clientbound::UpdateBlockEntity,
    ) {
        match block_update.nbt {
            None => {
                // NBT is null, so we need to remove the block entity
                world.clone()
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
                        world.clone().add_block_entity_action(
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

    /*
    fn on_block_entity_update_data(
        &mut self,
        _block_update: packet::play::clientbound::UpdateBlockEntity_Data,
    ) {
        // TODO: handle UpdateBlockEntity_Data for 1.7, decompress gzipped_nbt
    }*/

    /*
    fn on_sign_update(&mut self, mut update_sign: packet::play::clientbound::UpdateSign) {
        format::convert_legacy(&mut update_sign.line1);
        format::convert_legacy(&mut update_sign.line2);
        format::convert_legacy(&mut update_sign.line3);
        format::convert_legacy(&mut update_sign.line4);
        self.world.clone().write().unwrap()
            .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                update_sign.location,
                update_sign.line1,
                update_sign.line2,
                update_sign.line3,
                update_sign.line4,
            ))));
    }

    fn on_sign_update_u16(&mut self, mut update_sign: packet::play::clientbound::UpdateSign_u16) {
        format::convert_legacy(&mut update_sign.line1);
        format::convert_legacy(&mut update_sign.line2);
        format::convert_legacy(&mut update_sign.line3);
        format::convert_legacy(&mut update_sign.line4);
        self.world.clone().write().unwrap()
            .add_block_entity_action(world::BlockEntityAction::UpdateSignText(Box::new((
                Position::new(update_sign.x, update_sign.y as i32, update_sign.z),
                update_sign.line1,
                update_sign.line2,
                update_sign.line3,
                update_sign.line4,
            ))));
    }*/

    fn on_player_info_string(
        &mut self,
        _player_info: packet::play::clientbound::PlayerInfo_String,
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
/*
    fn on_player_info(&mut self, player_info: packet::play::clientbound::PlayerInfo) {
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
                    let mut players = players.write().unwrap();
                    let info = players.entry(uuid.clone()).or_insert(PlayerInfo {
                        name: name.clone(),
                        uuid,
                        skin_url: None,

                        display_name: display.clone(),
                        ping: ping.0,
                        gamemode: Gamemode::from_int(gamemode.0),
                    });
                    // Re-set the props of the player in case of dodgy server implementations
                    info.name = name;
                    info.display_name = display;
                    info.ping = ping.0;
                    info.gamemode = Gamemode::from_int(gamemode.0);
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
                        let skin_blob: serde_json::Value = match serde_json::from_slice(&skin_blob)
                        {
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
                            .clone().write().unwrap()
                            .get_component_mut_direct::<entity::player::PlayerModel>(
                                self.player.clone().write().unwrap().unwrap(),
                            )
                            .unwrap();
                        model.set_skin(info.skin_url.clone());
                    }
                }
                UpdateGamemode { uuid, gamemode } => {
                    if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                        info.gamemode = Gamemode::from_int(gamemode.0);
                    }
                }
                UpdateLatency { uuid, ping } => {
                    if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                        info.ping = ping.0;
                    }
                }
                UpdateDisplayName { uuid, display } => {
                    if let Some(info) = self.players.clone().write().unwrap().get_mut(&uuid) {
                        info.display_name = display;
                    }
                }
                Remove { uuid } => {
                    self.players.clone().write().unwrap().remove(&uuid);
                }
            }
        }
    }*/

    fn on_player_info_glob(
        entities: Arc<RwLock<Manager>>,
        players: Arc<RwLock<HashMap<protocol::UUID, PlayerInfo, BuildHasherDefault<FNVHash>>>>,
        internal_player: Arc<RwLock<Option<ecs::Entity>>>,
        puuid: UUID,
        player_info: packet::play::clientbound::PlayerInfo) {
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
                    let players = players.clone();
                    let mut players = players.write().unwrap();
                    let info = players.entry(uuid.clone()).or_insert(PlayerInfo {
                        name: name.clone(),
                        uuid,
                        skin_url: None,

                        display_name: display.clone(),
                        ping: ping.0,
                        gamemode: Gamemode::from_int(gamemode.0),
                    });
                    // Re-set the props of the player in case of dodgy server implementations
                    info.name = name;
                    info.display_name = display;
                    info.ping = ping.0;
                    info.gamemode = Gamemode::from_int(gamemode.0);
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
                        let skin_blob: serde_json::Value = match serde_json::from_slice(&skin_blob)
                        {
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
                    if info.uuid == puuid {
                        let model = entities
                            .clone().write().unwrap()
                            .get_component_mut_direct::<entity::player::PlayerModel>(
                                internal_player.clone().write().unwrap().unwrap(),
                            )
                            .unwrap();
                        model.set_skin(info.skin_url.clone());
                    }
                }
                UpdateGamemode { uuid, gamemode } => {
                    if let Some(info) = players.clone().write().unwrap().get_mut(&uuid) {
                        info.gamemode = Gamemode::from_int(gamemode.0);
                    }
                }
                UpdateLatency { uuid, ping } => {
                    if let Some(info) = players.clone().write().unwrap().get_mut(&uuid) {
                        info.ping = ping.0;
                    }
                }
                UpdateDisplayName { uuid, display } => {
                    if let Some(info) = players.clone().write().unwrap().get_mut(&uuid) {
                        info.display_name = display;
                    }
                }
                Remove { uuid } => {
                    players.clone().write().unwrap().remove(&uuid);
                }
            }
        }
    }

    fn on_servermessage_noposition(
        &mut self,
        m: packet::play::clientbound::ServerMessage_NoPosition,
    ) {
        self.on_servermessage(&m.message, None, None);
    }

    fn on_servermessage_position(&mut self, m: packet::play::clientbound::ServerMessage_Position) {
        self.on_servermessage(&m.message, Some(m.position), None);
    }

    fn on_servermessage_sender(&mut self, m: packet::play::clientbound::ServerMessage_Sender) {
        self.on_servermessage(&m.message, Some(m.position), Some(m.sender));
    }

    fn on_servermessage(
        &mut self,
        message: &format::Component,
        _position: Option<u8>,
        _sender: Option<protocol::UUID>,
    ) {
        info!("Received chat message: {}", message);
        self.received_chat_at.clone().write().unwrap().replace(Instant::now());
    }

    fn load_block_entities(&mut self, block_entities: Vec<Option<crate::nbt::NamedTag>>) {
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

    fn load_block_entities_glob(world: Arc<World>, block_entities: Vec<Option<crate::nbt::NamedTag>>) {
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
                Server::on_block_entity_update_glob(world.clone(), packet::play::clientbound::UpdateBlockEntity {
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

    /*
    fn on_chunk_data_biomes3d_varint(
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_Biomes3D_VarInt,
    ) {
        self.world.clone().write().unwrap()
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
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_Biomes3D_bool,
    ) {
        self.world.clone().write().unwrap()
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

    fn on_chunk_data_biomes3d(
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_Biomes3D,
    ) {
        self.world.clone().write().unwrap()
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

    fn on_chunk_data(&mut self, chunk_data: packet::play::clientbound::ChunkData) {
        self.world.clone().write().unwrap()
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

    fn on_chunk_data_heightmap(
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_HeightMap,
    ) {
        self.world.clone().write().unwrap()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
        self.load_block_entities(chunk_data.block_entities.data);
    }*/

    /*
    fn on_chunk_data_no_entities(
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_NoEntities,
    ) {
        self.world.clone().write().unwrap()
            .load_chunk19(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask.0 as u16,
                chunk_data.data.data,
            )
            .unwrap();
    }*/

    /*
    fn on_chunk_data_no_entities_u16(
        &mut self,
        chunk_data: packet::play::clientbound::ChunkData_NoEntities_u16,
    ) {
        let chunk_meta = vec![crate::protocol::packet::ChunkMeta {
            x: chunk_data.chunk_x,
            z: chunk_data.chunk_z,
            bitmask: chunk_data.bitmask,
        }];
        let skylight = false;
        self.world.clone().write().unwrap()
            .load_chunks18(chunk_data.new, skylight, &chunk_meta, chunk_data.data.data)
            .unwrap();
    }*/

    /*
    fn on_chunk_data_17(&mut self, chunk_data: packet::play::clientbound::ChunkData_17) {
        self.world.clone().write().unwrap()
            .load_chunk17(
                chunk_data.chunk_x,
                chunk_data.chunk_z,
                chunk_data.new,
                chunk_data.bitmask,
                chunk_data.add_bitmask,
                chunk_data.compressed_data.data,
            )
            .unwrap();
    }*/

    /*
    fn on_chunk_data_bulk(&mut self, bulk: packet::play::clientbound::ChunkDataBulk) {
        let new = true;
        self.world.clone().write().unwrap()
            .load_chunks18(
                new,
                bulk.skylight,
                &bulk.chunk_meta.data,
                bulk.chunk_data.to_vec(),
            )
            .unwrap();
    }*/

    /*
    fn on_chunk_data_bulk_17(&mut self, bulk: packet::play::clientbound::ChunkDataBulk_17) {
        self.world.clone().write().unwrap()
            .load_chunks17(
                bulk.chunk_column_count,
                bulk.data_length,
                bulk.skylight,
                &bulk.chunk_data_and_meta,
            )
            .unwrap();
    }*/

    /*
    fn on_chunk_unload(&mut self, chunk_unload: packet::play::clientbound::ChunkUnload) {
        self.world.clone().write().unwrap()
            .unload_chunk(chunk_unload.x, chunk_unload.z, &mut self.entities.clone().write().unwrap());
    }*/

    fn on_block_change(&mut self, location: Position, id: i32) {
        Server::on_block_change_in_world(self.world.clone(), location, id);
    }

    fn on_block_change_in_world(world_cont: Arc<World>, location: Position, id: i32) {
        let world = world_cont.clone();
        let modded_block_ids = world.modded_block_ids.clone();
        let block = world
            .id_map
            .by_vanilla_id(id as usize, modded_block_ids);
        drop(world);
        world_cont.clone().set_block(
            location,
            block,
        )
    }

    /*
    fn on_block_change_varint(
        &mut self,
        block_change: packet::play::clientbound::BlockChange_VarInt,
    ) {
        self.on_block_change(block_change.location, block_change.block_id.0)
    }*/

    /*
    fn on_block_change_u8(&mut self, block_change: packet::play::clientbound::BlockChange_u8) {
        self.on_block_change(
            crate::shared::Position::new(block_change.x, block_change.y as i32, block_change.z),
            (block_change.block_id.0 << 4) | (block_change.block_metadata as i32),
        );
    }*/

    /*
    fn on_multi_block_change_packed(
        &mut self,
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
            let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
            let block = self.world.clone().read().unwrap()
                .id_map
                .by_vanilla_id(block_raw_id as usize, modded_block_ids);
            self.world.clone().write().unwrap().set_block(
                Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32),
                block,
            );*/
            self.on_block_change(Position::new(sx + lx as i32, sy + ly as i32, sz + lz as i32), block_raw_id as i32);
        }
    }*/

    fn on_multi_block_change_varint(
        &mut self,
        block_change: packet::play::clientbound::MultiBlockChange_VarInt,
    ) {
        let ox = block_change.chunk_x << 4;
        let oz = block_change.chunk_z << 4;
        for record in block_change.records.data {
            /*let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
            let block = self.world.clone().read().unwrap()
                .id_map
                .by_vanilla_id(record.block_id.0 as usize, modded_block_ids);
            self.world.clone().write().unwrap().set_block(
                Position::new(
                    ox + (record.xz >> 4) as i32,
                    record.y as i32,
                    oz + (record.xz & 0xF) as i32,
                ),
                block,
            );*/
            self.on_block_change(Position::new(
                ox + (record.xz >> 4) as i32,
                record.y as i32,
                oz + (record.xz & 0xF) as i32,
            ), record.block_id.0 as i32);
        }
    }

    /*
    fn on_multi_block_change_u16(
        &mut self,
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
            let modded_block_ids = &self.world.clone().read().unwrap().modded_block_ids;
            let block = self.world.clone().read().unwrap()
                .id_map
                .by_vanilla_id(id as usize, modded_block_ids);
            self.world.write().unwrap().set_block(
                Position::new(x, y, z),
                block,
            );*/
            self.on_block_change(Position::new(x, y, z), id as i32);
        }
    }*/
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
