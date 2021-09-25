use crate::protocol::mapped_packet::play::clientbound::{Advancements, AcknowledgePlayerDigging, Animation, BlockAction, BlockBreakAnimation, BlockChange, BossBar, ChangeGameState, ConfirmTransaction, ChunkUnload, ChunkData, ChunkData_HeightMap, ChunkData_Biomes3D_i32, ChunkData_Biomes3D, ChunkData_Biomes3D_bool, ChunkData_NoEntities_u16, ChunkData_NoEntities, ChunkData_17, ChunkDataBulk_17, ChunkDataBulk, Camera, CoFHLib_SendUUID, CollectItem, CombatEvent};
use crate::protocol::mapped_packet::play::serverbound::{AdvancementTab, ChatMessage, ArmSwing, ClientStatus, ClientSettings, ConfirmTransactionServerbound, ClickWindow, ClickWindowButton, ClientAbilities, CloseWindow, CraftingBookData, CraftRecipeRequest};
macro_rules! state_mapped_packets {
     ($($state:ident $stateName:ident {
        $($dir:ident $dirName:ident {
            $(
                $(#[$attr:meta])*
                packet $name:ident {
                    $($(#[$fattr:meta])*field $field:ident: $field_type:ty, )+
                }
            )*
        })+
    })+) => {
        use crate::protocol::*;
        use std::io;

        #[derive(Debug)]
        pub enum MappedPacket {
        $(
            $(
                $(
        $name($state::$dir::$name),
                )*
            )+
        )+
        }

        $(
        pub mod $state {

            $(
            pub mod $dir {
                #![allow(unused_imports)]
                use crate::protocol::*;
                use std::io;
                use crate::format;
                use crate::nbt;
                use crate::types;
                use crate::item;
                use crate::shared::Position;


                $(
                    #[derive(Default, Debug)]
                    $(#[$attr])* pub struct $name {
                        $($(#[$fattr])* pub $field: $field_type),+,
                    }
                )*
            }
            )+
        }
        )+
    }
}

state_mapped_packets!(
    handshake Handshaking {
         serverbound Serverbound {
            /// Handshake is the first packet sent in the protocol.
            /// Its used for deciding if the request is a client
            /// is requesting status information about the server
            /// (MOTD, players etc) or trying to login to the server.
            ///
            /// The host and port fields are not used by the vanilla
            /// server but are there for virtual server hosting to
            /// be able to redirect a client to a target server with
            /// a single address + port.
            ///
            /// Some modified servers/proxies use the handshake field
            /// differently, packing information into the field other
            /// than the hostname due to the protocol not providing
            /// any system for custom information to be transfered
            /// by the client to the server until after login.
            packet Handshake {
                /// The protocol version of the connecting client
                field protocol_version: i32,
                /// The hostname the client connected to
                field host: String,
                /// The port the client connected to
                field port: u16,
                /// The next protocol state the client wants
                field next: i32,
            }
        }
        clientbound Clientbound {
        }
    }
    play Play {
        serverbound Serverbound {
            /// TeleportConfirm is sent by the client as a reply to a telport from
            /// the server.
            packet TeleportConfirm {
                field teleport_id: i32,
            }
            packet QueryBlockNBT {
                field transaction_id: i32,
                field location: Position,
            }
            packet SetDifficulty {
                field new_difficulty: u8,
            }
            /// TabComplete is sent by the client when the client presses tab in
            /// the chat box.
            packet TabComplete {
                field text: String,
                field assume_command: Option<bool>,
                field has_target: Option<bool>,
                field target: Option<Position>,
            }
            /// ChatMessage is sent by the client when it sends a chat message or
            /// executes a command (prefixed by '/').
            packet ChatMessage {
                field message: String,
            }
            /// ClientStatus is sent to update the client's status
            packet ClientStatus {
                field action_id: i32,
            }
            /// ClientSettings is sent by the client to update its current settings.
            packet ClientSettings {
                field locale: String,
                field view_distance: u8,
                field chat_mode: i32,
                field chat_colors: bool,
                field difficulty: Option<u8>,
                field displayed_skin_parts: u8,
                field main_hand: Option<i32>,
            }
            /// ConfirmTransactionServerbound is a reply to ConfirmTransaction.
            packet ConfirmTransactionServerbound {
                field id: u8,
                field action_number: i16,
                field accepted: bool,
            }
            /// EnchantItem is sent when the client enchants an item.
            packet EnchantItem {
                field id: u8,
                field enchantment: u8,
            }
            /// ClickWindowButton is used for clicking an enchantment, lectern, stonecutter, or loom.
            packet ClickWindowButton {
                field id: u8,
                field button: u8,
            }
            /// ClickWindow is sent when the client clicks in a window.
            packet ClickWindow {
                field id: u8,
                field slot: i16,
                field button: u8,
                field action_number: u16,
                field mode: i32,
                field clicked_item: Option<item::Stack>,
            }
            /// CloseWindow is sent when the client closes a window.
            packet CloseWindow {
                field id: u8,
            }
            /// PluginMessageServerbound is used for custom messages between the client
            /// and server. This is mainly for plugins/mods but vanilla has a few channels
            /// registered too.
            packet PluginMessageServerbound {
                field channel: String,
                field data: Vec<u8>,
            }
            packet EditBook {
                field new_book: Option<item::Stack>,
                field is_signing: bool,
                field hand: i32,
            }
            packet QueryEntityNBT {
                field transaction_id: i32,
                field entity_id: i32,
            }
            /// UseEntity is sent when the user interacts (right clicks) or attacks
            /// (left clicks) an entity.
            packet UseEntity_Sneakflag {
                field target_id: i32,
                field ty: i32,
                field target_x: Option<f32>,
                field target_y: Option<f32>,
                field target_z: Option<f32>,
                field hand: Option<i32>,
                field sneaking: Option<bool>,
            }
            /// Sent when Generate is pressed on the Jigsaw Block interface.
            packet GenerateStructure {
                field location: Position,
                field levels: i32,
                field keep_jigsaws: bool,
            }
            /// KeepAliveServerbound is sent by a client as a response to a
            /// KeepAliveClientbound. If the client doesn't reply the server
            /// may disconnect the client.
            packet KeepAliveServerbound {
                field id: i64,
            }
            packet LockDifficulty {
                field locked: bool,
            }
            /// PlayerPosition is used to update the player's position.
            packet PlayerPosition {
                field x: f64,
                field y: Option<f64>,
                field feet_y: Option<f64>,
                field head_y: Option<f64>,
                field z: f64,
                field on_ground: bool,
            }
            /// PlayerPositionLook is a combination of PlayerPosition and
            /// PlayerLook.
            packet PlayerPositionLook {
                field x: f64,
                field y: Option<f64>,
                field feet_y: Option<f64>,
                field head_y: Option<f64>,
                field z: f64,
                field yaw: f32,
                field pitch: f32,
                field on_ground: bool,
            }
            /// PlayerLook is used to update the player's rotation.
            packet PlayerLook {
                field yaw: f32,
                field pitch: f32,
                field on_ground: bool,
            }
            /// Player is used to update whether the player is on the ground or not.
            packet Player {
                field on_ground: bool,
            }
            /// Sent by the client when in a vehicle instead of the normal move packet.
            packet VehicleMove {
                field x: f64,
                field y: f64,
                field z: f64,
                field yaw: f32,
                field pitch: f32,
            }
            /// SteerBoat is used to visually update the boat paddles.
            packet SteerBoat {
                field left_paddle_turning: bool,
                field right_paddle_turning: bool,
            }
            packet PickItem {
                field slot_to_use: i32,
            }
            /// CraftRecipeRequest is sent when player clicks a recipe in the crafting book.
            packet CraftRecipeRequest {
                field window_id: u8,
                field recipe: i32,
                field make_all: bool,
            }
            /// ClientAbilities is used to modify the players current abilities.
            /// Currently flying is the only one
            packet ClientAbilities {
                field flags: u8,
                field flying_speed: Option<f32>,
                field walking_speed: Option<f32>,
            }
            /// PlayerDigging is sent when the client starts/stops digging a block.
            /// It also can be sent for droppping items and eating/shooting.
            packet PlayerDigging {
                field status: i32,
                field location: Position,
                field face: u8,
            }
            /// PlayerAction is sent when a player preforms various actions.
            packet PlayerAction {
                field entity_id: i32,
                field action_id: i32,
                field jump_boost: i32,
            }
            /// SteerVehicle is sent by the client when steers or preforms an action
            /// on a vehicle.
            packet SteerVehicle {
                field sideways: f32,
                field forward: f32,
                field flags: Option<u8>,
                field jump: Option<bool>,
                field unmount: Option<bool>,
            }
            /// CraftingBookData is sent when the player interacts with the crafting book.
            packet CraftingBookData {
                field action: i32,
                field recipe_id: i32,
                field crafting_book_open: bool,
                field crafting_filter: bool,
            }
            /// SetDisplayedRecipe replaces CraftingBookData, type 0.
            packet SetDisplayedRecipe {
                field recipe_id: String,
            }
            /// SetRecipeBookState replaces CraftingBookData, type 1.
            packet SetRecipeBookState {
                field book_id: i32, // TODO: enum, 0: crafting, 1: furnace, 2: blast furnace, 3: smoker
                field book_open: bool,
                field filter_active: bool,
            }
            packet NameItem {
                field item_name: String,
            }
            /// ResourcePackStatus informs the server of the client's current progress
            /// in activating the requested resource pack
            packet ResourcePackStatus {
                field hash: Option<String>,
                field result: i32,
            }
            // TODO: Document
            packet AdvancementTab {
                field action: i32,
                field tab_id: String,
            }
            packet SelectTrade {
                field selected_slot: i32,
            }
            packet SetBeaconEffect {
                field primary_effect: i32,
                field secondary_effect: i32,
            }
            /// HeldItemChange is sent when the player changes the currently active
            /// hotbar slot.
            packet HeldItemChange {
                field slot: i16,
            }
            packet UpdateCommandBlock {
                field location: Position,
                field command: String,
                field mode: i32,
                field flags: u8,
            }
            packet UpdateCommandBlockMinecart {
                field entity_id: i32,
                field command: String,
                field track_output: bool,
            }
            /// CreativeInventoryAction is sent when the client clicks in the creative
            /// inventory. This is used to spawn items in creative.
            packet CreativeInventoryAction {
                field slot: i16,
                field clicked_item: Option<item::Stack>,
            }
            packet UpdateJigsawBlock_Joint {
                field location: Position,
                field name: String,
                field target: String,
                field pool: String,
                field final_state: String,
                field joint_type: String,
            }
            packet UpdateJigsawBlock_Type {
                field location: Position,
                field attachment_type: String,
                field target_pool: String,
                field final_state: String,
            }
            packet UpdateStructureBlock {
                field location: Position,
                field action: i32,
                field mode: i32,
                field name: String,
                field offset_x: i8,
                field offset_y: i8,
                field offset_z: i8,
                field size_x: i8,
                field size_y: i8,
                field size_z: i8,
                field mirror: i32,
                field rotation: i32,
                field metadata: String,
                field integrity: f32,
                field seed: VarLong,
                field flags: i8,
            }
            /// SetSign sets the text on a sign after placing it.
            packet SetSign {
                field location: Position,
                field line1: String,
                field line2: String,
                field line3: String,
                field line4: String,
            }
            /// ArmSwing is sent by the client when the player left clicks (to swing their
            /// arm).
            packet ArmSwing {
                field hand: Option<i32>,
                field entity_id: Option<i32>,
                field animation: Option<u8>,
            }
            /// SpectateTeleport is sent by clients in spectator mode to teleport to a player.
            packet SpectateTeleport {
                field target: UUID,
            }
            /// PlayerBlockPlacement is sent when the client tries to place a block.
            packet PlayerBlockPlacement_f32 {
                field location: Position,
                field face: i32,
                field hand: Option<i32>,
                field hand_item: Option<item::Stack>,
                field cursor_x: f32,
                field cursor_y: f32,
                field cursor_z: f32,
                field inside_block: Option<bool>, // 1.14 added insideblock
            }
            /// UseItem is sent when the client tries to use an item.
            packet UseItem {
                field hand: i32,
            }
        }
        clientbound Clientbound {
            /// SpawnObject is used to spawn an object or vehicle into the world when it
            /// is in range of the client.
            packet SpawnObject {
                field entity_id: i32,
                field uuid: Option<UUID>,
                field ty: i32, // 1.14 changed u8 to i32
                field x: f64,
                field y: f64,
                field z: f64,
                field pitch: i8,
                field yaw: i8,
                field data: i32,
                field velocity_x: i16,
                field velocity_y: i16,
                field velocity_z: i16,
            }
            /// SpawnExperienceOrb spawns a single experience orb into the world when
            /// it is in range of the client. The count controls the amount of experience
            /// gained when collected.
            packet SpawnExperienceOrb {
                field entity_id: i32,
                field x: f64,
                field y: f64,
                field z: f64,
                field count: i16,
            }
            /// SpawnGlobalEntity spawns an entity which is visible from anywhere in the
            /// world. Currently only used for lightning.
            packet SpawnGlobalEntity {
                field entity_id: i32,
                field ty: u8,
                field x: f64,
                field y: f64,
                field z: f64,
            }
            /// SpawnMob is used to spawn a living entity into the world when it is in
            /// range of the client.
            packet SpawnMob {
                field entity_id: i32,
                field uuid: Option<UUID>,
                field ty: i32,
                field x: f64,
                field y: f64,
                field z: f64,
                field yaw: i8,
                field pitch: i8,
                field head_pitch: i8,
                field velocity_x: i16,
                field velocity_y: i16,
                field velocity_z: i16,
                field metadata: Option<types::Metadata>,
            }
            /// SpawnPainting spawns a painting into the world when it is in range of
            /// the client. The title effects the size and the texture of the painting.
            packet SpawnPainting {
                field entity_id: i32,
                field uuid: Option<UUID>,
                field motive: Option<i32>,
                field title: Option<String>,
                field location: Position,
                field direction: i32,
            }
            /// SpawnPlayer is used to spawn a player when they are in range of the client.
            /// This packet alone isn't enough to display the player as the skin and username
            /// information is in the player information packet.
            packet SpawnPlayer {
                field entity_id: i32,
                field uuid: Option<UUID>,
                field uuid_str: Option<String>,
                field name: Option<String>,
                field properties: Option<Vec<packet::SpawnProperty>>,
                field x: f64,
                field y: f64,
                field z: f64,
                field yaw: i8,
                field pitch: i8,
                field current_item: Option<u16>,
                field metadata: Option<types::Metadata>,
            }

            /// Animation is sent by the server to play an animation on a specific entity.
            packet Animation {
                field entity_id: i32,
                field animation_id: u8,
            }
            /// Statistics is used to update the statistics screen for the client.
            packet Statistics {
                field statistices: Vec<packet::Statistic>,
            }
            /// BlockBreakAnimation is used to create and update the block breaking
            /// animation played when a player starts digging a block.
            packet BlockBreakAnimation {
                field entity_id: i32,
                field location: Position,
                field stage: i8,
            }
            /// UpdateBlockEntity updates the nbt tag of a block entity in the
            /// world.
            packet UpdateBlockEntity {
                field location: Position,
                field action: u8,
                field nbt: Option<nbt::NamedTag>,
                field data_length: Option<i16>,
                field gzipped_nbt: Option<Vec<u8>>,
            }
            /// BlockAction triggers different actions depending on the target block.
            packet BlockAction {
                field location: Position,
                field byte1: u8,
                field byte2: u8,
                field block_type: i32,
            }
            /// BlockChange is used to update a single block on the client.
            packet BlockChange {
                field location: Position,
                field block_id: i32,
                field block_metadata: Option<u8>,
            }
            /// BossBar displays and/or changes a boss bar that is displayed on the
            /// top of the client's screen. This is normally used for bosses such as
            /// the ender dragon or the wither.
            packet BossBar {
                field uuid: UUID,
                field action: i32,
                field title: format::Component,
                field health: f32,
                field color: i32,
                field style: i32,
                field flags: u8,
            }
            /// ServerDifficulty changes the displayed difficulty in the client's menu
            /// as well as some ui changes for hardcore.
            packet ServerDifficulty {
                field difficulty: u8,
                field locked: Option<bool>,
            }
            /// TabCompleteReply is sent as a reply to a tab completion request.
            /// The matches should be possible completions for the command/chat the
            /// player sent.
            packet TabCompleteReply {
                field matches: Vec<String>,
            }
            packet DeclareCommands {
                field nodes: Vec<packet::CommandNode>,
                field root_index: i32,
            }
            /// ServerMessage is a message sent by the server. It could be from a player
            /// or just a system message. The Type field controls the location the
            /// message is displayed at and when the message is displayed.
            packet ServerMessage {
                field message: format::Component,
                /// 0 - Chat message, 1 - System message, 2 - Action bar message
                field position: Option<u8>,
                field sender: Option<UUID>,
            }
            /// MultiBlockChange is used to update a batch of blocks in a single packet.
            packet MultiBlockChange_Packed {
                field chunk_section_pos: u64,
                field no_trust_edges: bool,
                field records: Vec<VarLong>,
            }
            packet MultiBlockChange_i32 {
                field chunk_x: i32,
                field chunk_z: i32,
                field records: Option<Vec<packet::BlockChangeRecord>>,
                field record_count: Option<u16>,
                field data_size: Option<i32>,
                field data: Option<Vec<u8>>,
            }
            /// ConfirmTransaction notifies the client whether a transaction was successful
            /// or failed (e.g. due to lag).
            packet ConfirmTransaction {
                field id: u8,
                field action_number: i16,
                field accepted: bool,
            }
            /// WindowClose forces the client to close the window with the given id,
            /// e.g. a chest getting destroyed.
            packet WindowClose {
                field id: u8,
            }
            /// WindowOpen tells the client to open the inventory window of the given
            /// type. The ID is used to reference the instance of the window in
            /// other packets.
            packet WindowOpen {
                field id: i32,
                field ty: Option<i32>,
                field ty_name: Option<String>,
                field title: format::Component,
                field slot_count: Option<u8>,
                field use_provided_window_title: Option<bool>,
                field entity_id: Option<i32>,
            }
            packet WindowOpenHorse {
                field window_id: u8,
                field number_of_slots: i32,
                field entity_id: i32,
            }
            /// WindowItems sets every item in a window.
            packet WindowItems {
                field id: u8,
                field items: Vec<Option<item::Stack>>,
            }
            /// WindowProperty changes the value of a property of a window. Properties
            /// vary depending on the window type.
            packet WindowProperty {
                field id: u8,
                field property: i16,
                field value: i16,
            }
            /// WindowSetSlot changes an itemstack in one of the slots in a window.
            packet WindowSetSlot {
                field id: i8,
                field slot: i16,
                field item: Option<item::Stack>,
            }
            /// SetCooldown disables a set item (by id) for the set number of ticks
            packet SetCooldown {
                field item_id: i32,
                field ticks: i32,
            }
            /// PluginMessageClientbound is used for custom messages between the client
            /// and server. This is mainly for plugins/mods but vanilla has a few channels
            /// registered too.
            packet PluginMessageClientbound {
                field channel: String,
                field data: Option<Vec<u8>>,
            }
            /// Plays a sound by name on the client
            packet NamedSoundEffect {
                field name: String,
                field category: Option<i32>,
                field x: i32,
                field y: i32,
                field z: i32,
                field volume: f32,
                field pitch: f32,
            }
            /// Disconnect causes the client to disconnect displaying the passed reason.
            packet Disconnect {
                field reason: format::Component,
            }
            /// EntityAction causes an entity to preform an action based on the passed
            /// id.
            packet EntityAction {
                field entity_id: i32,
                field action_id: u8,
            }
            /// Explosion is sent when an explosion is triggered (tnt, creeper etc).
            /// This plays the effect and removes the effected blocks.
            packet Explosion {
                field x: f32,
                field y: f32,
                field z: f32,
                field radius: f32,
                field records: Vec<packet::ExplosionRecord>,
                field velocity_x: f32,
                field velocity_y: f32,
                field velocity_z: f32,
            }
            /// ChunkUnload tells the client to unload the chunk at the specified
            /// position.
            packet ChunkUnload {
                field x: i32,
                field z: i32,
            }
            /// SetCompression updates the compression threshold.
            packet SetCompression {
                field threshold: i32,
            }
            /// ChangeGameState is used to modify the game's state like gamemode or
            /// weather.
            packet ChangeGameState {
                field reason: u8,
                field value: f32,
            }
            /// KeepAliveClientbound is sent by a server to check if the
            /// client is still responding and keep the connection open.
            /// The client should reply with the KeepAliveServerbound
            /// packet setting ID to the same as this one.
            packet KeepAliveClientbound {
                field id: i64,
            }
            /// ChunkData sends or updates a single chunk on the client. If New is set
            /// then biome data should be sent too.
            packet ChunkData_Biomes3D_i32 {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: i32,
                field heightmaps: Option<nbt::NamedTag>,
                field biomes: Vec<VarInt>,
                field data: Vec<u8>,
                field block_entities: Vec<Option<nbt::NamedTag>>,
            }
            packet ChunkData_Biomes3D_bool {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field ignore_old_data: bool,
                field bitmask: i32,
                field heightmaps: Option<nbt::NamedTag>,
                field biomes: Biomes3D,
                field data: Vec<u8>,
                field block_entities: Vec<Option<nbt::NamedTag>>,
            }
            packet ChunkData_Biomes3D {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: i32,
                field heightmaps: Option<nbt::NamedTag>,
                field biomes: Biomes3D,
                field data: Vec<u8>,
                field block_entities: Vec<Option<nbt::NamedTag>>,
            }
            packet ChunkData_HeightMap {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: i32,
                field heightmaps: Option<nbt::NamedTag>,
                field data: Vec<u8>,
                field block_entities: Vec<Option<nbt::NamedTag>>,
            }
            packet ChunkData {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: i32,
                field data: Vec<u8>,
                field block_entities: Vec<Option<nbt::NamedTag>>,
            }
            packet ChunkData_NoEntities {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: i32,
                field data: Vec<u8>,
            }
            packet ChunkData_NoEntities_u16 {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: u16,
                field data: Vec<u8>,
            }
            packet ChunkData_17 {
                field chunk_x: i32,
                field chunk_z: i32,
                field new: bool,
                field bitmask: u16,
                field add_bitmask: u16,
                field compressed_data: Vec<u8>,
            }
            packet ChunkDataBulk {
                field skylight: bool,
                field chunk_meta: Vec<packet::ChunkMeta>,
                field chunk_data: Vec<u8>,
            }
            packet ChunkDataBulk_17 {
                field chunk_column_count: u16,
                field data_length: i32,
                field skylight: bool,
                field chunk_data_and_meta: Vec<u8>,
            }
            /// Effect plays a sound effect or particle at the target location with the
            /// volume (of sounds) being relative to the player's position unless
            /// DisableRelative is set to true.
            packet Effect {
                field effect_id: i32,
                field location: Position,
                field data: i32,
                field disable_relative: bool,
            }
            /// Particle spawns particles at the target location with the various
            /// modifiers.
            packet Particle {
                field particle_id: Option<i32>,
                field particle_name: Option<String>,
                field long_distance: bool,
                field x: f64,
                field y: f64,
                field z: f64,
                field offset_x: f32,
                field offset_y: f32,
                field offset_z: f32,
                field speed: f32,
                field count: i32,
                field block_state: Option<i32>,
                field red: Option<f32>,
                field green: Option<f32>,
                field blue: Option<f32>,
                field scale: Option<f32>,
                field item: Option<nbt::NamedTag>,
                field data1: Option<i32>,
                field data2: Option<i32>,
            }
            /// JoinGame is sent after completing the login process. This
            /// sets the initial state for the client.
            packet JoinGame_WorldNames_IsHard {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// Whether hardcore mode is enabled
                field is_hardcore: bool,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The previous gamemode of the client
                field previous_gamemode: u8,
                /// Identifiers for all worlds on the server
                field world_names: Vec<String>,
                /// Represents a dimension registry
                field dimension_codec: Option<nbt::NamedTag>,
                /// The dimension the client is starting in
                field dimension: Option<nbt::NamedTag>,
                /// The world being spawned into
                field world_name: String,
                /// Truncated SHA-256 hash of world's seed
                field hashed_seed: i64,
                /// The max number of players on the server
                field max_players: i32,
                /// The render distance (2-32)
                field view_distance: i32,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
                /// Whether to prompt or immediately respawn
                field enable_respawn_screen: bool,
                /// Whether the world is in debug mode
                field is_debug: bool,
                /// Whether the world is a superflat world
                field is_flat: bool,
            }
            packet JoinGame_WorldNames {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The previous gamemode of the client
                field previous_gamemode: u8,
                /// Identifiers for all worlds on the server
                field world_names: Vec<String>,
                /// Represents a dimension registry
                field dimension_codec: Option<nbt::NamedTag>,
                /// The dimension the client is starting in
                field dimension: String,
                /// The world being spawned into
                field world_name: String,
                /// Truncated SHA-256 hash of world's seed
                field hashed_seed: i64,
                /// The max number of players on the server
                field max_players: u8,
                /// The render distance (2-32)
                field view_distance: i32,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
                /// Whether to prompt or immediately respawn
                field enable_respawn_screen: bool,
                /// Whether the world is in debug mode
                field is_debug: bool,
                /// Whether the world is a superflat world
                field is_flat: bool,
            }
            packet JoinGame_HashedSeed_Respawn {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The dimension the client is starting in
                field dimension: i32,
                /// Truncated SHA-256 hash of world's seed
                field hashed_seed: i64,
                /// The max number of players on the server
                field max_players: u8,
                /// The level type of the server
                field level_type: String,
                /// The render distance (2-32)
                field view_distance: i32,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
                /// Whether to prompt or immediately respawn
                field enable_respawn_screen: bool,
            }
            packet JoinGame_i32_ViewDistance {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The dimension the client is starting in
                field dimension: i32,
                /// The max number of players on the server
                field max_players: u8,
                /// The level type of the server
                field level_type: String,
                /// The render distance (2-32)
                field view_distance: i32,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
            }
            packet JoinGame_i32 {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The dimension the client is starting in
                field dimension: i32,
                /// The difficuilty setting for the server
                field difficulty: u8,
                /// The max number of players on the server
                field max_players: u8,
                /// The level type of the server
                field level_type: String,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
            }
            packet JoinGame_i8 {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The dimension the client is starting in
                field dimension: i8,
                /// The difficuilty setting for the server
                field difficulty: u8,
                /// The max number of players on the server
                field max_players: u8,
                /// The level type of the server
                field level_type: String,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: bool,
            }
            packet JoinGame_i8_NoDebug {
                field entity_id: i32,
                field gamemode: u8,
                field dimension: i8,
                field difficulty: u8,
                field max_players: u8,
                field level_type: String,
            }
            /// Maps updates a single map's contents
            packet Maps {
                field item_damage: i32,
                field scale: Option<i8>,
                field tracking_position: Option<bool>,
                field locked: Option<bool>,
                field icons: Option<Vec<packet::MapIcon>>,
                field columns: Option<u8>,
                field rows: Option<u8>,
                field x: Option<u8>,
                field z: Option<u8>,
                field data: Option<Vec<u8>>,
            }
            /// EntityMove moves the entity with the id by the offsets provided.
            packet EntityMove {
                field entity_id: i32,
                field delta_x: f64,
                field delta_y: f64,
                field delta_z: f64,
                field on_ground: Option<bool>,
            }
            /// EntityLookAndMove is a combination of EntityMove and EntityLook.
            packet EntityLookAndMove {
                field entity_id: i32,
                field delta_x: f64,
                field delta_y: f64,
                field delta_z: f64,
                field yaw: i8,
                field pitch: i8,
                field on_ground: Option<bool>,
            }
            /// EntityLook rotates the entity to the new angles provided.
            packet EntityLook {
                field entity_id: i32,
                field yaw: i8,
                field pitch: i8,
                field on_ground: Option<bool>,
            }
            /// Entity does nothing. It is a result of subclassing used in Minecraft.
            packet Entity {
                field entity_id: i32,
            }
            /// EntityUpdateNBT updates the entity named binary tag.
            packet EntityUpdateNBT {
                field entity_id: i32,
                field nbt: Option<nbt::NamedTag>,
            }
            /// Teleports the player's vehicle
            packet VehicleTeleport {
                field x: f64,
                field y: f64,
                field z: f64,
                field yaw: f32,
                field pitch: f32,
            }
            /// Opens the book GUI.
            packet OpenBook {
                field hand: i32,
            }
            /// SignEditorOpen causes the client to open the editor for a sign so that
            /// it can write to it. Only sent in vanilla when the player places a sign.
            packet SignEditorOpen {
                field location: Position,
            }
            /// CraftRecipeResponse is a response to CraftRecipeRequest, notifies the UI.
            packet CraftRecipeResponse {
                field window_id: u8,
                field recipe: i32,
            }
            /// PlayerAbilities is used to modify the players current abilities. Flying,
            /// creative, god mode etc.
            packet PlayerAbilities {
                field flags: u8,
                field flying_speed: f32,
                field walking_speed: f32,
            }
            /// CombatEvent is used for... you know, I never checked. I have no
            /// clue.
            packet CombatEvent {
                field event: i32,
                field direction: Option<i32>,
                field player_id: Option<i32>,
                field entity_id: Option<i32>,
                field message: Option<format::Component>,
            }
            /// PlayerInfo is sent by the server for every player connected to the server
            /// to provide skin and username information as well as ping and gamemode info.
            packet PlayerInfo {
                field inner: packet::PlayerInfoData,
            }
            packet PlayerInfo_String {
                field name: String,
                field online: bool,
                field ping: u16,
            }
            packet FacePlayer {
                field feet_eyes: i32,
                field target_x: f64,
                field target_y: f64,
                field target_z: f64,
                field is_entity: bool,
                field entity_id: Option<i32>,
                field entity_feet_eyes: Option<i32>,
            }
            /// TeleportPlayer is sent to change the player's position. The client is expected
            /// to reply to the server with the same positions as contained in this packet
            /// otherwise will reject future packets.
            packet TeleportPlayer {
                field x: f64,
                field y: Option<f64>,
                field eyes_y: Option<f64>,
                field z: f64,
                field yaw: f32,
                field pitch: f32,
                field flags: Option<u8>,
                field teleport_id: Option<i32>,
                field on_ground: Option<bool>,
            }
            /// EntityUsedBed is sent by the server when a player goes to bed.
            packet EntityUsedBed {
                field entity_id: i32,
                field location: Position,
            }
            packet UnlockRecipes {
                field action: i32,
                field crafting_book_open: bool,
                field filtering_craftable: bool,
                field smelting_book_open: Option<bool>,
                field filtering_smeltable: Option<bool>,
                field blast_furnace_open: Option<bool>,
                field filtering_blast_furnace: Option<bool>,
                field smoker_open: Option<bool>,
                field filtering_smoker: Option<bool>,
                field recipe_ids: Vec<i32>,
                field recipe_ids2: Vec<i32>,
                field recipe_ids_str: Vec<String>,
                field recipe_ids_str2: Vec<String>,
            }
            /// EntityDestroy destroys the entities with the ids in the provided slice.
            packet EntityDestroy {
                field entity_ids: Vec<i32>,
            }
            /// EntityRemoveEffect removes an effect from an entity.
            packet EntityRemoveEffect {
                field entity_id: i32,
                field effect_id: i8,
            }
            /// ResourcePackSend causes the client to check its cache for the requested
            /// resource packet and download it if its missing. Once the resource pack
            /// is obtained the client will use it.
            packet ResourcePackSend {
                field url: String,
                field hash: String,
            }
            /// Respawn is sent to respawn the player after death or when they move worlds.
            packet Respawn_Gamemode {
                field dimension_tag: Option<nbt::NamedTag>,
                field dimension_name: Option<String>,
                field world_name: Option<String>,
                field dimension: Option<i32>,
                field hashed_seed: Option<i64>,
                field difficulty: Option<u8>,
                field gamemode: u8,
                field level_type: Option<String>,
                field previous_gamemode: Option<u8>,
                field is_debug: Option<bool>,
                field is_flat: Option<bool>,
                field copy_metadata: Option<bool>,
            }
            /// EntityHeadLook rotates an entity's head to the new angle.
            packet EntityHeadLook {
                field entity_id: i32,
                field head_yaw: i8,
            }
            packet EntityStatus {
                field entity_id: i32,
                field entity_status: i8,
            }
            packet NBTQueryResponse {
                field transaction_id: i32,
                field nbt: Option<nbt::NamedTag>,
            }
            /// SelectAdvancementTab indicates the client should switch the advancement tab.
            packet SelectAdvancementTab {
                field has_id: bool,
                field tab_id: String,
            }
            /// WorldBorder configures the world's border.
            packet WorldBorder {
                field action: i32,
                field old_radius: Option<f64>,
                field new_radius: Option<f64>,
                field speed: Option<VarLong>,
                field x: Option<f64>,
                field z: Option<f64>,
                field portal_boundary: Option<i32>,
                field warning_time: Option<i32>,
                field warning_blocks: Option<i32>,
            }
            /// Camera causes the client to spectate the entity with the passed id.
            /// Use the player's id to de-spectate.
            packet Camera {
                field target_id: i32,
            }
            /// SetCurrentHotbarSlot changes the player's currently selected hotbar item.
            packet SetCurrentHotbarSlot {
                field slot: u8,
            }
            /// UpdateViewPosition is used to determine what chunks should be remain loaded.
            packet UpdateViewPosition {
                field chunk_x: i32,
                field chunk_z: i32,
            }
            /// UpdateViewDistance is sent by the integrated server when changing render distance.
            packet UpdateViewDistance {
                field view_distance: i32,
            }
            /// ScoreboardDisplay is used to set the display position of a scoreboard.
            packet ScoreboardDisplay {
                field position: u8,
                field name: String,
            }
            /// EntityMetadata updates the metadata for an entity.
            packet EntityMetadata {
                field entity_id: i32,
                field metadata: types::Metadata,
            }
            /// EntityAttach attaches to entities together, either by mounting or leashing.
            /// -1 can be used at the EntityID to deattach.
            packet EntityAttach {
                field entity_id: i32,
                field vehicle: i32,
                field leash: Option<bool>,
            }
            /// EntityVelocity sets the velocity of an entity in 1/8000 of a block
            /// per a tick.
            packet EntityVelocity {
                field entity_id: i32,
                field velocity_x: i16,
                field velocity_y: i16,
                field velocity_z: i16,
            }
            /// EntityEquipment is sent to display an item on an entity, like a sword
            /// or armor. Slot 0 is the held item and slots 1 to 4 are boots, leggings
            /// chestplate and helmet respectively.
            packet EntityEquipment_Array {
                field entity_id: i32,
                field equipments: packet::EntityEquipments,
            }
            packet EntityEquipment_Single {
                field entity_id: i32,
                field slot: i32,
                field item: Option<item::Stack>,
            }
            /// SetExperience updates the experience bar on the client.
            packet SetExperience {
                field experience_bar: f32,
                field level: i32,
                field total_experience: i32,
            }
            /// UpdateHealth is sent by the server to update the player's health and food.
            packet UpdateHealth {
                field health: f32,
                field food: i32,
                field food_saturation: f32,
            }
            /// ScoreboardObjective creates/updates a scoreboard objective.
            packet ScoreboardObjective {
                field name: String,
                field mode: Option<u8>,
                field value: String,
                field ty_str: Option<String>,
                field ty: Option<u8>,
            }
            /// SetPassengers mounts entities to an entity
            packet SetPassengers {
                field entity_id: i32,
                field passengers: Vec<i32>,
            }
            /// Teams creates and updates teams
            packet Teams {
                field name: String,
                field mode: u8,
                field display_name: Option<String>,
                field flags: Option<u8>,
                field name_tag_visibility: Option<String>,
                field collision_rule: Option<String>,
                field formatting: Option<i32>,
                field prefix: Option<String>,
                field suffix: Option<String>,
                field players: Option<Vec<String>>,
                field color: Option<i8>,
                field data: Option<Vec<u8>>,
            }
            /// UpdateScore is used to update or remove an item from a scoreboard
            /// objective.
            packet UpdateScore {
                field name: String,
                field action: u8,
                field object_name: String,
                field value: Option<i32>,
            }
            /// SpawnPosition is sent to change the player's current spawn point. Currently
            /// only used by the client for the compass.
            packet SpawnPosition {
                field location: Position,
            }
            /// TimeUpdate is sent to sync the world's time to the client, the client
            /// will manually tick the time itself so this doesn't need to sent repeatedly
            /// but if the server or client has issues keeping up this can fall out of sync
            /// so it is a good idea to send this now and again
            packet TimeUpdate {
                field world_age: i64,
                field time_of_day: i64,
            }
            packet StopSound {
                field flags: u8,
                field source: Option<i32>,
                field sound: Option<String>,
            }
            /// Title configures an on-screen title.
            packet Title {
                field action: i32,
                field title: Option<format::Component>,
                field sub_title: Option<format::Component>,
                field action_bar_text: Option<String>,
                field fade_in: Option<i32>,
                field fade_stay: Option<i32>,
                field fade_out: Option<i32>,
                field fade_in_comp: Option<format::Component>,
                field fade_stay_comp: Option<format::Component>,
                field fade_out_comp: Option<format::Component>,
            }
            /// UpdateSign sets or changes the text on a sign.
            packet UpdateSign {
                field location: Position,
                field line1: format::Component,
                field line2: format::Component,
                field line3: format::Component,
                field line4: format::Component,
            }
            /// SoundEffect plays the named sound at the target location.
            packet SoundEffect {
                field name: i32,
                field category: i32,
                field x: i32,
                field y: i32,
                field z: i32,
                field volume: f32,
                field pitch: f32,
            }
            /// Plays a sound effect from an entity.
            packet EntitySoundEffect {
                field sound_id: i32,
                field sound_category: i32,
                field entity_id: i32,
                field volume: f32,
                field pitch: f32,
            }
            /// PlayerListHeaderFooter updates the header/footer of the player list.
            packet PlayerListHeaderFooter {
                field header: format::Component,
                field footer: format::Component,
            }
            /// CollectItem causes the collected item to fly towards the collector. This
            /// does not destroy the entity.
            packet CollectItem {
                field collected_entity_id: i32,
                field collector_entity_id: i32,
                field number_of_items: Option<i32>,
            }
            /// EntityTeleport teleports the entity to the target location. This is
            /// sent if the entity moves further than EntityMove allows.
            packet EntityTeleport {
                field entity_id: i32,
                field x: f64,
                field y: f64,
                field z: f64,
                field yaw: i8,
                field pitch: i8,
                field on_ground: Option<bool>,
            }
            packet Advancements {
                field data: Vec<u8>,
                /* TODO: fix parsing modded advancements 1.12.2 (e.g. SevTech Ages)
                 * see https://github.com/iceiix/stevenarella/issues/148
                field reset_clear: bool,
                field mapping: Vec<packet::Advancement>,
                field identifiers: Vec<String>,
                field progress: Vec<packet::AdvancementProgress>,
                */
            }
            /// EntityProperties updates the properties for an entity.
            packet EntityProperties {
                field entity_id: i32,
                field properties: Vec<mapped_packet::EntityProperty>,
            }
            /// EntityEffect applies a status effect to an entity for a given duration.
            packet EntityEffect {
                field entity_id: i32,
                field effect_id: i8,
                field amplifier: i8,
                field duration: i32,
                field hide_particles: Option<bool>,
            }
            packet DeclareRecipes {
                field recipes: Vec<packet::Recipe>,
            }
            packet Tags {
                field block_tags: Vec<packet::Tags>,
                field item_tags: Vec<packet::Tags>,
                field fluid_tags: Vec<packet::Tags>,
                field entity_tags: Option<Vec<packet::Tags>>,
            }
            packet AcknowledgePlayerDigging {
                field location: Position,
                field block: i32,
                field status: i32,
                field successful: bool,
            }
            packet UpdateLight {
                field chunk_x: i32,
                field chunk_z: i32,
                field trust_edges: Option<bool>,
                field sky_light_mask: i32,
                field block_light_mask: i32,
                field empty_sky_light_mask: i32,
                field light_arrays: Vec<u8>,
            }
            packet TradeList {
                field id: i32,
                field trades: Vec<packet::Trade>,
                field villager_level: i32,
                field experience: i32,
                field is_regular_villager: bool,
                field can_restock: Option<bool>,
            }
            packet CoFHLib_SendUUID {
                field player_uuid: UUID,
            }
       }
    }
    login Login {
        serverbound Serverbound {
            /// LoginStart is sent immeditately after switching into the login
            /// state. The passed username is used by the server to authenticate
            /// the player in online mode.
            packet LoginStart {
                field username: String,
            }
            /// EncryptionResponse is sent as a reply to EncryptionRequest. All
            /// packets following this one must be encrypted with AES/CFB8
            /// encryption.
            packet EncryptionResponse {
                /// The key for the AES/CFB8 cipher encrypted with the
                /// public key
                field shared_secret: Vec<u8>,
                /// The verify token from the request encrypted with the
                /// public key
                field verify_token: Vec<u8>,
            }
            packet EncryptionResponse_i16 {
                field shared_secret: Vec<u8>,
                field verify_token: Vec<u8>,
            }
            packet LoginPluginResponse {
                field message_id: i32,
                field successful: bool,
                field data: Vec<u8>,
            }
        }
        clientbound Clientbound {
            /// LoginDisconnect is sent by the server if there was any issues
            /// authenticating the player during login or the general server
            /// issues (e.g. too many players).
            packet LoginDisconnect {
                field reason: format::Component,
            }
            /// EncryptionRequest is sent by the server if the server is in
            /// online mode. If it is not sent then its assumed the server is
            /// in offline mode.
            packet EncryptionRequest {
                /// Generally empty, left in from legacy auth
                /// but is still used by the client if provided
                field server_id: String,
                /// A RSA Public key serialized in x.509 PRIX format
                field public_key: Vec<u8>,
                /// Token used by the server to verify encryption is working
                /// correctly
                field verify_token: Vec<u8>,
            }
            packet EncryptionRequest_i16 {
                field server_id: String,
                field public_key: Vec<u8>,
                field verify_token: Vec<u8>,
            }
            /// LoginSuccess is sent by the server if the player successfully
            /// authenicates with the session servers (online mode) or straight
            /// after LoginStart (offline mode).
            packet LoginSuccess_String {
                /// String encoding of a uuid (with hyphens)
                field uuid: String,
                field username: String,
            }
            packet LoginSuccess_UUID {
                field uuid: UUID,
                field username: String,
            }
            /// SetInitialCompression sets the compression threshold during the
            /// login state.
            packet SetInitialCompression {
                /// Threshold where a packet should be sent compressed
                field threshold: i32,
            }
            packet LoginPluginRequest {
                field message_id: i32,
                field channel: String,
                field data: Vec<u8>,
            }
        }
    }
    status Status {
        serverbound Serverbound {
            /// StatusRequest is sent by the client instantly after
            /// switching to the Status protocol state and is used
            /// to signal the server to send a StatusResponse to the
            /// client
            packet StatusRequest {
                field empty: (),
            }
            /// StatusPing is sent by the client after recieving a
            /// StatusResponse. The client uses the time from sending
            /// the ping until the time of recieving a pong to measure
            /// the latency between the client and the server.
            packet StatusPing {
                field ping: i64,
            }
        }
        clientbound Clientbound {
            /// StatusResponse is sent as a reply to a StatusRequest.
            /// The Status should contain a json encoded structure with
            /// version information, a player sample, a description/MOTD
            /// and optionally a favicon.
            //
            /// The structure is as follows
            ///
            /// ```json
            /// {
            ///     "version": {
            ///         "name": "1.8.3",
            ///         "protocol": 47,
            ///     },
            ///     "players": {
            ///         "max": 20,
            ///         "online": 1,
            ///         "sample": [
            ///            packet  {"name": "Thinkofdeath", "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"}
            ///         ]
            ///     },
            ///     "description": "Hello world",
            ///     "favicon": "data:image/png;base64,<data>"
            /// }
            /// ```
            packet StatusResponse {
                field status: String,
            }
            /// StatusPong is sent as a reply to a StatusPing.
            /// The Time field should be exactly the same as the
            /// one sent by the client.
            packet StatusPong {
                field ping: i64,
            }
       }
    }
);

#[derive(Debug, Default)]
pub struct EntityProperty {
    pub key: String,
    pub value: f64,
    pub modifiers: Vec<PropertyModifier>,
}

pub trait MappablePacket {

    fn map(&self) -> MappedPacket;

}


impl MappablePacket for packet::Packet {

    fn map(self) -> MappedPacket {
       match self {
           packet::Packet::Advancements(advancements) => {
               mapped_packet::MappedPacket::Advancements(Advancements {
                   data: advancements.data,
               })
           }
           packet::Packet::AcknowledgePlayerDigging(digging) => {
               mapped_packet::MappedPacket::AcknowledgePlayerDigging(AcknowledgePlayerDigging {
                   location: digging.location,
                   block: digging.block.0,
                   status: digging.status.0,
                   successful: digging.successful,
               })
           }
           packet::Packet::AdvancementTab(advancement) => {
               mapped_packet::MappedPacket::AdvancementTab(AdvancementTab {
                   action: advancement.action.0,
                   tab_id: advancement.tab_id,
               })
           }
           packet::Packet::Animation(animation) => {
               mapped_packet::MappedPacket::Animation(Animation {
                   entity_id: animation.entity_id.0,
                   animation_id: animation.animation_id,
               })
           }
           packet::Packet::ArmSwing(arm_swing) => {
               mapped_packet::MappedPacket::ArmSwing(ArmSwing {
                   hand: Some(arm_swing.hand.0),
                   entity_id: None,
                   animation: None,
               })
           }
           packet::Packet::ArmSwing_Handsfree(arm_swing) => {
               mapped_packet::MappedPacket::ArmSwing(ArmSwing {
                   hand: None,
                   entity_id: None,
                   animation: None,
               })
           }
           packet::Packet::ArmSwing_Handsfree_ID(arm_swing) => {
               mapped_packet::MappedPacket::ArmSwing(ArmSwing {
                   hand: None,
                   entity_id: Some(arm_swing.entity_id),
                   animation: Some(arm_swing.animation),
               })
           }
           packet::Packet::BlockAction(block_action) => {
               mapped_packet::MappedPacket::BlockAction(BlockAction {
                   location: block_action.location,
                   byte1: block_action.byte1,
                   byte2: block_action.byte2,
                   block_type: block_action.block_type.0,
               })
           }
           packet::Packet::BlockAction_u16(block_action) => {
               mapped_packet::MappedPacket::BlockAction(BlockAction {
                   location: Position::new(block_action.x, block_action.y as i32, block_action.z),
                   byte1: block_action.byte1,
                   byte2: block_action.byte2,
                   block_type: block_action.block_type.0,
               })
           }
           packet::Packet::BlockBreakAnimation(break_animation) => {
               mapped_packet::MappedPacket::BlockBreakAnimation(BlockBreakAnimation {
                   entity_id: break_animation.entity_id.0,
                   location: break_animation.location,
                   stage: break_animation.stage,
               })
           }
           packet::Packet::BlockBreakAnimation_i32(break_animation) => {
               mapped_packet::MappedPacket::BlockBreakAnimation(BlockBreakAnimation {
                   entity_id: break_animation.entity_id.0,
                   location: Position::new(break_animation.x, break_animation.y, break_animation.z),
                   stage: break_animation.stage,
               })
           }
           packet::Packet::BlockChange_u8(block_change) => {
               mapped_packet::MappedPacket::BlockChange(BlockChange {
                   location: Position::new(block_change.x, block_change.y as i32, block_change.z),
                   block_id: block_change.block_id.0,
                   block_metadata: Some(block_change.block_metadata),
               })
           }
           packet::Packet::BlockChange_VarInt(block_change) => {
               mapped_packet::MappedPacket::BlockChange(BlockChange {
                   location: block_change.location,
                   block_id: block_change.block_id.0,
                   block_metadata: None,
               })
           }
           packet::Packet::BossBar(boss_bar) => {
               mapped_packet::MappedPacket::BossBar(BossBar {
                   uuid: boss_bar.uuid,
                   action: boss_bar.action.0,
                   title: boss_bar.title,
                   health: boss_bar.health,
                   color: boss_bar.color.0,
                   style: boss_bar.style.0,
                   flags: boss_bar.flags,
               })
           }
           packet::Packet::ChatMessage(chat_msg) => {
               mapped_packet::MappedPacket::ChatMessage(ChatMessage {
                   message: chat_msg.message,
               })
           }
           packet::Packet::ChangeGameState(change_game_state) => {
               mapped_packet::MappedPacket::ChangeGameState(ChangeGameState {
                   reason: change_game_state.reason,
                   value: change_game_state.value,
               })
           }
           packet::Packet::ClientStatus(client_status) => {
               mapped_packet::MappedPacket::ClientStatus(ClientStatus {
                   action_id: client_status.action_id.0,
               })
           }
           packet::Packet::ClientStatus_u8(client_status) => {
               mapped_packet::MappedPacket::ClientStatus(ClientStatus {
                   action_id: client_status.action_id as i32,
               })
           }
           packet::Packet::ClientSettings(client_settings) => {
               mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                   locale: client_settings.locale,
                   view_distance: client_settings.view_distance,
                   chat_mode: client_settings.chat_mode.0,
                   chat_colors: client_settings.chat_colors,
                   difficulty: None,
                   displayed_skin_parts: client_settings.displayed_skin_parts,
                   main_hand: Some(client_settings.main_hand.0),
               })
           }
           packet::Packet::ClientSettings_u8(client_settings) => {
               mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                   locale: client_settings.locale,
                   view_distance: client_settings.view_distance,
                   chat_mode: client_settings.chat_mode.0,
                   chat_colors: client_settings.chat_colors,
                   difficulty: None,
                   displayed_skin_parts: client_settings.displayed_skin_parts,
                   main_hand: Some(client_settings.main_hand.0),
               })
           }
           packet::Packet::ClientSettings_u8_Handsfree(client_settings) => {
               mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                   locale: client_settings.locale,
                   view_distance: client_settings.view_distance,
                   chat_mode: client_settings.chat_mode.0,
                   chat_colors: client_settings.chat_colors,
                   difficulty: None,
                   displayed_skin_parts: client_settings.displayed_skin_parts,
                   main_hand: None,
               })
           }
           packet::Packet::ClientSettings_u8_Handsfree_Difficulty(client_settings) => {
               mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                   locale: client_settings.locale,
                   view_distance: client_settings.view_distance,
                   chat_mode: client_settings.chat_mode.0,
                   chat_colors: client_settings.chat_colors,
                   difficulty: Some(client_settings.difficulty),
                   displayed_skin_parts: client_settings.displayed_skin_parts,
                   main_hand: None,
               })
           }
           packet::Packet::ConfirmTransactionServerbound(confirm_transaction) => {
               mapped_packet::MappedPacket::ConfirmTransactionServerbound(ConfirmTransactionServerbound {
                   id: confirm_transaction.id,
                   action_number: confirm_transaction.action_number,
                   accepted: confirm_transaction.accepted,
               })
           }
           packet::Packet::ConfirmTransaction(confirm_transaction) => {
               mapped_packet::MappedPacket::ConfirmTransaction(ConfirmTransaction {
                   id: confirm_transaction.id,
                   action_number: confirm_transaction.action_number,
                   accepted: confirm_transaction.accepted,
               })
           }
           packet::Packet::ChunkUnload(chunk_unload) => {
               mapped_packet::MappedPacket::ChunkUnload(ChunkUnload {
                   x: chunk_unload.x,
                   z: chunk_unload.z,
               })
           }
           packet::Packet::ChunkData(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData(ChunkData {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask.0,
                   data: chunk_data.data.data,
                   block_entities: chunk_data.block_entities.data,
               })
           }
           packet::Packet::ChunkData_HeightMap(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_HeightMap(ChunkData_HeightMap {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask.0,
                   heightmaps: chunk_data.heightmaps,
                   data: chunk_data.data.data,
                   block_entities: chunk_data.block_entities.data,
               })
           }
           packet::Packet::ChunkData_Biomes3D_VarInt(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_Biomes3D_i32(ChunkData_Biomes3D_i32 {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask.0,
                   heightmaps: chunk_data.heightmaps,
                   biomes: chunk_data.biomes.data,
                   data: chunk_data.data.data,
                   block_entities: chunk_data.block_entities.data,
               })
           }
           packet::Packet::ChunkData_Biomes3D(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_Biomes3D(ChunkData_Biomes3D {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask.0,
                   heightmaps: chunk_data.heightmaps,
                   biomes: chunk_data.biomes.data,
                   data: chunk_data.data.data,
                   block_entities: chunk_data.block_entities.data,
               })
           }
           packet::Packet::ChunkData_Biomes3D_bool(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_Biomes3D_bool(ChunkData_Biomes3D_bool {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   ignore_old_data: chunk_data.ignore_old_data,
                   bitmask: chunk_data.bitmask.0,
                   heightmaps: chunk_data.heightmaps,
                   biomes: chunk_data.biomes.data,
                   data: chunk_data.data.data,
                   block_entities: chunk_data.block_entities.data,
               })
           }
           packet::Packet::ChunkData_17(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_Biomes3D_i32(ChunkData_17 {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask,
                   add_bitmask: chunk_data.add_bitmask,
                   compressed_data: chunk_data.compressed_data.data,
               })
           }
           packet::Packet::ChunkData_NoEntities(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_NoEntities(ChunkData_NoEntities {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask.0,
                   data: chunk_data.data.data,
               })
           }
           packet::Packet::ChunkData_NoEntities_u16(chunk_data) => {
               mapped_packet::MappedPacket::ChunkData_NoEntities_u16(ChunkData_NoEntities_u16 {
                   chunk_x: chunk_data.chunk_x,
                   chunk_z: chunk_data.chunk_z,
                   new: chunk_data.new,
                   bitmask: chunk_data.bitmask,
                   data: chunk_data.data.data,
               })
           }
           packet::Packet::ChunkDataBulk_17(chunk_data) => {
               mapped_packet::MappedPacket::ChunkDataBulk_17(ChunkDataBulk_17 {
                   chunk_column_count: chunk_data.chunk_column_count,
                   data_length: chunk_data.data_length,
                   skylight: chunk_data.skylight,
                   chunk_data_and_meta: chunk_data.chunk_data_and_meta,
               })
           }
           packet::Packet::ChunkDataBulk(chunk_data) => {
               mapped_packet::MappedPacket::ChunkDataBulk(ChunkDataBulk {
                   skylight: chunk_data.skylight,
                   chunk_meta: chunk_data.chunk_meta,
                   chunk_data: chunk_data.chunk_data,
               })
           }
           packet::Packet::Camera(camera) => {
               mapped_packet::MappedPacket::Camera(Camera {
                   target_id: camera.target_id.0,
               })
           }
           packet::Packet::ClickWindow(click_window) => {
               mapped_packet::MappedPacket::ClickWindow(ClickWindow {
                   id: click_window.id,
                   slot: click_window.slot,
                   button: click_window.button,
                   action_number: click_window.action_number,
                   mode: click_window.mode.0,
                   clicked_item: click_window.clicked_item,
               })
           }
           packet::Packet::ClickWindow_u8(click_window) => {
               mapped_packet::MappedPacket::ClickWindow(ClickWindow {
                   id: click_window.id,
                   slot: click_window.slot,
                   button: click_window.button,
                   action_number: click_window.action_number,
                   mode: click_window.mode,
                   clicked_item: click_window.clicked_item,
               })
           }
           packet::Packet::ClickWindowButton(click_window_button) => {
               mapped_packet::MappedPacket::ClickWindowButton(ClickWindowButton {
                   id: click_window_button.id,
                   button: click_window_button.button,
               })
           }
           packet::Packet::ClientAbilities_f32(client_abilities) => {
               mapped_packet::MappedPacket::ClientAbilities(ClientAbilities {
                   flags: client_abilities.flags,
                   flying_speed: Some(client_abilities.flying_speed),
                   walking_speed: Some(client_abilities.walking_speed),
               })
           }
           packet::Packet::ClientAbilities_u8(client_abilities) => {
               mapped_packet::MappedPacket::ClientAbilities(ClientAbilities {
                   flags: client_abilities.flags,
                   flying_speed: None,
                   walking_speed: None,
               })
           }
           packet::Packet::CloseWindow(close_window) => {
               mapped_packet::MappedPacket::CloseWindow(CloseWindow {
                   id: close_window.id,
               })
           }
           packet::Packet::CoFHLib_SendUUID(send_uuid) => {
               mapped_packet::MappedPacket::CoFHLib_SendUUID(CoFHLib_SendUUID {
                   player_uuid: send_uuid.player_uuid,
               })
           }
           packet::Packet::CollectItem(collect_item) => {
               mapped_packet::MappedPacket::CollectItem(CollectItem {
                   collected_entity_id: collect_item.collected_entity_id.0,
                   collector_entity_id: collect_item.collector_entity_id.0,
                   number_of_items: Some(collect_item.number_of_items.0),
               })
           }
           packet::Packet::CollectItem_nocount(collect_item) => {
               mapped_packet::MappedPacket::CollectItem(CollectItem {
                   collected_entity_id: collect_item.collected_entity_id.0,
                   collector_entity_id: collect_item.collector_entity_id.0,
                   number_of_items: None,
               })
           }
           packet::Packet::CollectItem_nocount_i32(collect_item) => {
               mapped_packet::MappedPacket::CollectItem(CollectItem {
                   collected_entity_id: collect_item.collected_entity_id,
                   collector_entity_id: collect_item.collector_entity_id,
                   number_of_items: None,
               })
           }
           packet::Packet::CombatEvent(combat_event) => {
               mapped_packet::MappedPacket::CombatEvent(CombatEvent {
                   event: combat_event.event.0,
                   direction: combat_event.direction.map(|x| x.0),
                   player_id: combat_event.player_id.map(|x| x.0),
                   entity_id: combat_event.entity_id,
                   message: combat_event.message,
               })
           }
           packet::Packet::CraftingBookData(crafting_book) => {
               mapped_packet::MappedPacket::CraftingBookData(CraftingBookData {
                   action: crafting_book.action.0,
                   recipe_id: crafting_book.recipe_id,
                   crafting_book_open: crafting_book.crafting_book_open,
                   crafting_filter: crafting_book.crafting_filter,
               })
           }
           packet::Packet::CraftRecipeRequest(craft_recipe_request) => {
               mapped_packet::MappedPacket::CraftRecipeRequest(CraftRecipeRequest {

               })
           }
       }
    }

}


/*
           packet::Packet::Advancements(advancements) => {
               mapped_packet::MappedPacket::Advancements(Advancements {

               })
           }
*/