use crate::protocol::mapped_packet::handshake::serverbound::Handshake;
use crate::protocol::mapped_packet::login::clientbound::{
    EncryptionRequest, LoginDisconnect, LoginPluginRequest, LoginSuccess_String, LoginSuccess_UUID,
    SetInitialCompression,
};
use crate::protocol::mapped_packet::login::serverbound::{
    EncryptionResponse, LoginPluginResponse, LoginStart,
};
use crate::protocol::mapped_packet::play::clientbound::{
    AcknowledgePlayerDigging, Advancements, Animation, BlockAction, BlockBreakAnimation,
    BlockChange, BossBar, Camera, ChangeGameState, ChunkData, ChunkDataBulk, ChunkDataBulk_17,
    ChunkData_17, ChunkData_Biomes3D, ChunkData_Biomes3D_bool, ChunkData_Biomes3D_i32,
    ChunkData_HeightMap, ChunkData_NoEntities, ChunkData_NoEntities_u16, ChunkUnload,
    CoFHLib_SendUUID, CollectItem, CombatEvent, ConfirmTransaction, CraftRecipeResponse,
    DeclareCommands, DeclareRecipes, Disconnect, Effect, Entity, EntityAction, EntityAttach,
    EntityDestroy, EntityEffect, EntityEquipment_Array, EntityEquipment_Single, EntityHeadLook,
    EntityLook, EntityLookAndMove, EntityMetadata, EntityMove, EntityProperties,
    EntityRemoveEffect, EntitySoundEffect, EntityStatus, EntityTeleport, EntityUpdateNBT,
    EntityUsedBed, EntityVelocity, Explosion, FacePlayer, JoinGame_HashedSeed_Respawn,
    JoinGame_WorldNames, JoinGame_WorldNames_IsHard, JoinGame_i32, JoinGame_i32_ViewDistance,
    JoinGame_i8, JoinGame_i8_NoDebug, KeepAliveClientbound, Maps, MultiBlockChange,
    NBTQueryResponse, NamedSoundEffect, OpenBook, Particle, PlayerAbilities, PlayerInfo,
    PlayerInfo_String, PlayerListHeaderFooter, PluginMessageClientbound, ResourcePackSend, Respawn,
    ScoreboardDisplay, ScoreboardObjective, SelectAdvancementTab, ServerDifficulty, ServerMessage,
    SetCompression, SetCooldown, SetCurrentHotbarSlot, SetExperience, SetPassengers,
    SignEditorOpen, SoundEffect, SpawnExperienceOrb, SpawnGlobalEntity, SpawnMob, SpawnObject,
    SpawnPainting, SpawnPlayer, SpawnPosition, Statistics, StopSound, TabCompleteReply, Tags,
    Teams, TeleportPlayer, TimeUpdate, Title, TradeList, UnlockRecipes, UpdateBlockEntity,
    UpdateHealth, UpdateLight, UpdateScore, UpdateSign, UpdateViewDistance, UpdateViewPosition,
    VehicleTeleport, WindowClose, WindowItems, WindowOpen, WindowOpenHorse, WindowProperty,
    WindowSetSlot, WorldBorder,
};
use crate::protocol::mapped_packet::play::serverbound::{
    AdvancementTab, ArmSwing, ChatMessage, ClickWindow, ClickWindowButton, ClientAbilities,
    ClientSettings, ClientStatus, CloseWindow, ConfirmTransactionServerbound, CraftRecipeRequest,
    CraftingBookData, CreativeInventoryAction, EditBook, EnchantItem, GenerateStructure,
    HeldItemChange, KeepAliveServerbound, LockDifficulty, NameItem, PickItem, Player, PlayerAction,
    PlayerBlockPlacement, PlayerDigging, PlayerLook, PlayerPosition, PlayerPositionLook,
    PluginMessageServerbound, QueryBlockNBT, QueryEntityNBT, ResourcePackStatus, SelectTrade,
    SetBeaconEffect, SetDifficulty, SetDisplayedRecipe, SetRecipeBookState, SetSign,
    SpectateTeleport, SteerBoat, SteerVehicle, TabComplete, TeleportConfirm, UpdateCommandBlock,
    UpdateCommandBlockMinecart, UpdateJigsawBlock_Joint, UpdateJigsawBlock_Type,
    UpdateStructureBlock, UseEntity, UseItem, VehicleMove,
};
use crate::protocol::mapped_packet::status::clientbound::{StatusPong, StatusResponse};
use crate::protocol::mapped_packet::status::serverbound::{StatusPing, StatusRequest};
use crate::protocol::packet::Hand;
use crate::protocol::packet::PropertyModifier;
use std::io::Cursor;

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
                field main_hand: Option<Hand>,
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
                field hand: Hand,
            }
            packet QueryEntityNBT {
                field transaction_id: i32,
                field entity_id: i32,
            }
            /// UseEntity is sent when the user interacts (right clicks) or attacks
            /// (left clicks) an entity.
            packet UseEntity {
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
                field z: f64,
                field feet_y: Option<f64>,
                field head_y: Option<f64>,
                field on_ground: bool,
            }
            /// PlayerPositionLook is a combination of PlayerPosition and
            /// PlayerLook.
            packet PlayerPositionLook {
                field x: f64,
                field y: Option<f64>,
                field z: f64,
                field feet_y: Option<f64>,
                field head_y: Option<f64>,
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
            packet UpdateJigsawBlock_Type { // TODO: Check if this can be merged!
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
                field seed: i64,
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
                field hand: Option<Hand>,
                field entity_id: Option<i32>,
                field animation: Option<u8>,
            }
            /// SpectateTeleport is sent by clients in spectator mode to teleport to a player.
            packet SpectateTeleport {
                field target: UUID,
            }
            /// PlayerBlockPlacement is sent when the client tries to place a block.
            packet PlayerBlockPlacement {
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
            /// The block id is the actual block id combined with its metadata
            /// which is stored in the first 4 bits of this i32.
            packet BlockChange {
                field location: Position,
                field block_id: i32,
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
            packet MultiBlockChange {
                field chunk_x: i32,
                field chunk_y: Option<i32>,
                field chunk_z: i32,
                field no_trust_edges: Option<bool>,
                field records: Vec<mapped_packet::BlockChangeRecord>,
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
                field data: Vec<u8>,
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
                field biomes: Vec<i32>,
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
                field long_distance: Option<bool>,
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
            packet JoinGame {
                /// The entity id the client will be referenced by
                field entity_id: i32,
                /// Whether hardcore mode is enabled
                field is_hardcore: Option<bool>,
                /// The starting gamemode of the client
                field gamemode: u8,
                /// The previous gamemode of the client
                field previous_gamemode: Option<u8>,
                /// Identifiers for all worlds on the server
                field world_names: Option<Vec<String>>,
                /// Represents a dimension registry
                field dimension_codec: Option<nbt::NamedTag>,
                /// The dimension the client is starting in
                field dimension: Option<nbt::NamedTag>,
                field dimension_id: Option<i32>, // an alternative to dimension
                /// The difficuilty setting for the server
                field difficulty: Option<u8>,
                /// The level type of the server
                field level_type: Option<String>,
                /// The world being spawned into
                field world_name: Option<String>,
                /// Truncated SHA-256 hash of world's seed
                field hashed_seed: Option<i64>,
                /// The max number of players on the server
                field max_players: i32,
                /// The render distance (2-32)
                field view_distance: i32,
                /// Whether the client should reduce the amount of debug
                /// information it displays in F3 mode
                field reduced_debug_info: Option<bool>,
                /// Whether to prompt or immediately respawn
                field enable_respawn_screen: Option<bool>,
                /// Whether the world is in debug mode
                field is_debug: Option<bool>,
                /// Whether the world is a superflat world
                field is_flat: Option<bool>,
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
                field hand: Hand,
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
                field y: f64,
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
                field recipe_ids: Option<Vec<i32>>,
                field recipe_ids2: Option<Vec<i32>>,
                field recipe_ids_str: Option<Vec<String>>,
                field recipe_ids_str2: Option<Vec<String>>,
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
            packet Respawn {
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
                field speed: Option<i64>,
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

impl Default for MappedPacket {
    fn default() -> Self {
        panic!("This function is not meant to be used, it is only used to make `MappedPacket` visible to the outside world.")
    }
}

#[derive(Debug, Default)]
pub struct EntityProperty {
    pub key: String,
    pub value: f64,
    pub modifiers: Vec<PropertyModifier>,
}

pub trait MappablePacket {
    fn map(self) -> MappedPacket;
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
                    hand: Some(Hand::from(arm_swing.hand.0)),
                    entity_id: None,
                    animation: None,
                })
            }
            packet::Packet::ArmSwing_Handsfree(_arm_swing) => {
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
                    location: Position::new(
                        break_animation.x,
                        break_animation.y,
                        break_animation.z,
                    ),
                    stage: break_animation.stage,
                })
            }
            packet::Packet::BlockChange_u8(block_change) => {
                mapped_packet::MappedPacket::BlockChange(BlockChange {
                    location: Position::new(block_change.x, block_change.y as i32, block_change.z),
                    block_id: (block_change.block_id.0 << 4) | (block_change.block_metadata as i32),
                })
            }
            packet::Packet::BlockChange_VarInt(block_change) => {
                mapped_packet::MappedPacket::BlockChange(BlockChange {
                    location: block_change.location,
                    block_id: block_change.block_id.0,
                })
            }
            packet::Packet::BossBar(boss_bar) => mapped_packet::MappedPacket::BossBar(BossBar {
                uuid: boss_bar.uuid,
                action: boss_bar.action.0,
                title: boss_bar.title,
                health: boss_bar.health,
                color: boss_bar.color.0,
                style: boss_bar.style.0,
                flags: boss_bar.flags,
            }),
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
                    main_hand: Some(Hand::from(client_settings.main_hand.0)),
                })
            }
            packet::Packet::ClientSettings_u8(client_settings) => {
                mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                    locale: client_settings.locale,
                    view_distance: client_settings.view_distance,
                    chat_mode: client_settings.chat_mode as i32,
                    chat_colors: client_settings.chat_colors,
                    difficulty: None,
                    displayed_skin_parts: client_settings.displayed_skin_parts,
                    main_hand: Some(Hand::from(client_settings.main_hand.0)),
                })
            }
            packet::Packet::ClientSettings_u8_Handsfree(client_settings) => {
                mapped_packet::MappedPacket::ClientSettings(ClientSettings {
                    locale: client_settings.locale,
                    view_distance: client_settings.view_distance,
                    chat_mode: client_settings.chat_mode as i32,
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
                    chat_mode: client_settings.chat_mode as i32,
                    chat_colors: client_settings.chat_colors,
                    difficulty: Some(client_settings.difficulty),
                    displayed_skin_parts: client_settings.displayed_skin_parts,
                    main_hand: None,
                })
            }
            packet::Packet::ConfirmTransactionServerbound(confirm_transaction) => {
                mapped_packet::MappedPacket::ConfirmTransactionServerbound(
                    ConfirmTransactionServerbound {
                        id: confirm_transaction.id,
                        action_number: confirm_transaction.action_number,
                        accepted: confirm_transaction.accepted,
                    },
                )
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
                    biomes: chunk_data.biomes.data.iter().map(|x| x.0).collect(),
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
                    biomes: chunk_data.biomes,
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
                    biomes: chunk_data.biomes,
                    data: chunk_data.data.data,
                    block_entities: chunk_data.block_entities.data,
                })
            }
            packet::Packet::ChunkData_17(chunk_data) => {
                mapped_packet::MappedPacket::ChunkData_17(ChunkData_17 {
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
                    chunk_meta: chunk_data.chunk_meta.data,
                    chunk_data: chunk_data.chunk_data,
                })
            }
            packet::Packet::Camera(camera) => mapped_packet::MappedPacket::Camera(Camera {
                target_id: camera.target_id.0,
            }),
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
                    mode: click_window.mode as i32,
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
                    window_id: craft_recipe_request.window_id,
                    recipe: craft_recipe_request.recipe.0,
                    make_all: craft_recipe_request.make_all,
                })
            }
            packet::Packet::CraftRecipeResponse(craft_recipe_response) => {
                mapped_packet::MappedPacket::CraftRecipeResponse(CraftRecipeResponse {
                    window_id: craft_recipe_response.window_id,
                    recipe: craft_recipe_response.recipe.0,
                })
            }
            packet::Packet::CreativeInventoryAction(creative_inventory_action) => {
                mapped_packet::MappedPacket::CreativeInventoryAction(CreativeInventoryAction {
                    slot: creative_inventory_action.slot,
                    clicked_item: creative_inventory_action.clicked_item,
                })
            }
            packet::Packet::Disconnect(disconnect) => {
                mapped_packet::MappedPacket::Disconnect(Disconnect {
                    reason: disconnect.reason,
                })
            }
            packet::Packet::DeclareCommands(declare_commands) => {
                mapped_packet::MappedPacket::DeclareCommands(DeclareCommands {
                    nodes: declare_commands.nodes.data,
                    root_index: declare_commands.root_index.0,
                })
            }
            packet::Packet::DeclareRecipes(declare_recipes) => {
                mapped_packet::MappedPacket::DeclareRecipes(DeclareRecipes {
                    recipes: declare_recipes.recipes.data,
                })
            }
            packet::Packet::Entity(entity) => mapped_packet::MappedPacket::Entity(Entity {
                entity_id: entity.entity_id.0,
            }),
            packet::Packet::Entity_i32(entity) => mapped_packet::MappedPacket::Entity(Entity {
                entity_id: entity.entity_id,
            }),
            packet::Packet::EntityHeadLook(head_look) => {
                mapped_packet::MappedPacket::EntityHeadLook(EntityHeadLook {
                    entity_id: head_look.entity_id.0,
                    head_yaw: head_look.head_yaw,
                })
            }
            packet::Packet::EntityHeadLook_i32(head_look) => {
                mapped_packet::MappedPacket::EntityHeadLook(EntityHeadLook {
                    entity_id: head_look.entity_id,
                    head_yaw: head_look.head_yaw,
                })
            }
            packet::Packet::EntityVelocity(velocity) => {
                mapped_packet::MappedPacket::EntityVelocity(EntityVelocity {
                    entity_id: velocity.entity_id.0,
                    velocity_x: velocity.velocity_x,
                    velocity_y: velocity.velocity_y,
                    velocity_z: velocity.velocity_z,
                })
            }
            packet::Packet::EntityVelocity_i32(velocity) => {
                mapped_packet::MappedPacket::EntityVelocity(EntityVelocity {
                    entity_id: velocity.entity_id,
                    velocity_x: velocity.velocity_x,
                    velocity_y: velocity.velocity_y,
                    velocity_z: velocity.velocity_z,
                })
            }
            packet::Packet::EntityLookAndMove_i16(look_and_move) => {
                mapped_packet::MappedPacket::EntityLookAndMove(EntityLookAndMove {
                    entity_id: look_and_move.entity_id.0,
                    delta_x: From::from(look_and_move.delta_x),
                    delta_y: From::from(look_and_move.delta_y),
                    delta_z: From::from(look_and_move.delta_z),
                    yaw: look_and_move.yaw,
                    pitch: look_and_move.pitch,
                    on_ground: Some(look_and_move.on_ground),
                })
            }
            packet::Packet::EntityLookAndMove_i8(look_and_move) => {
                mapped_packet::MappedPacket::EntityLookAndMove(EntityLookAndMove {
                    entity_id: look_and_move.entity_id.0,
                    delta_x: From::from(look_and_move.delta_x),
                    delta_y: From::from(look_and_move.delta_y),
                    delta_z: From::from(look_and_move.delta_z),
                    yaw: look_and_move.yaw,
                    pitch: look_and_move.pitch,
                    on_ground: Some(look_and_move.on_ground),
                })
            }
            packet::Packet::EntityLookAndMove_i8_i32_NoGround(look_and_move) => {
                mapped_packet::MappedPacket::EntityLookAndMove(EntityLookAndMove {
                    entity_id: look_and_move.entity_id,
                    delta_x: From::from(look_and_move.delta_x),
                    delta_y: From::from(look_and_move.delta_y),
                    delta_z: From::from(look_and_move.delta_z),
                    yaw: look_and_move.yaw,
                    pitch: look_and_move.pitch,
                    on_ground: None,
                })
            }
            packet::Packet::EntityLook_i32_NoGround(look) => {
                mapped_packet::MappedPacket::EntityLook(EntityLook {
                    entity_id: look.entity_id,
                    yaw: look.yaw,
                    pitch: look.pitch,
                    on_ground: None,
                })
            }
            packet::Packet::EntityLook_VarInt(look) => {
                mapped_packet::MappedPacket::EntityLook(EntityLook {
                    entity_id: look.entity_id.0,
                    yaw: look.yaw,
                    pitch: look.pitch,
                    on_ground: Some(look.on_ground),
                })
            }
            packet::Packet::EntityTeleport_f64(teleport) => {
                mapped_packet::MappedPacket::EntityTeleport(EntityTeleport {
                    entity_id: teleport.entity_id.0,
                    x: teleport.x,
                    y: teleport.y,
                    z: teleport.z,
                    yaw: teleport.yaw,
                    pitch: teleport.pitch,
                    on_ground: Some(teleport.on_ground),
                })
            }
            packet::Packet::EntityTeleport_i32(teleport) => {
                mapped_packet::MappedPacket::EntityTeleport(EntityTeleport {
                    entity_id: teleport.entity_id.0,
                    x: From::from(teleport.x),
                    y: From::from(teleport.y),
                    z: From::from(teleport.z),
                    yaw: teleport.yaw,
                    pitch: teleport.pitch,
                    on_ground: Some(teleport.on_ground),
                })
            }
            packet::Packet::EntityTeleport_i32_i32_NoGround(teleport) => {
                mapped_packet::MappedPacket::EntityTeleport(EntityTeleport {
                    entity_id: teleport.entity_id,
                    x: From::from(teleport.x),
                    y: From::from(teleport.y),
                    z: From::from(teleport.z),
                    yaw: teleport.yaw,
                    pitch: teleport.pitch,
                    on_ground: None,
                })
            }
            packet::Packet::EntityMove_i16(entity_move) => {
                mapped_packet::MappedPacket::EntityMove(EntityMove {
                    entity_id: entity_move.entity_id.0,
                    delta_x: From::from(entity_move.delta_x),
                    delta_y: From::from(entity_move.delta_y),
                    delta_z: From::from(entity_move.delta_z),
                    on_ground: Some(entity_move.on_ground),
                })
            }
            packet::Packet::EntityMove_i8(entity_move) => {
                mapped_packet::MappedPacket::EntityMove(EntityMove {
                    entity_id: entity_move.entity_id.0,
                    delta_x: From::from(entity_move.delta_x),
                    delta_y: From::from(entity_move.delta_y),
                    delta_z: From::from(entity_move.delta_z),
                    on_ground: Some(entity_move.on_ground),
                })
            }
            packet::Packet::EntityMove_i8_i32_NoGround(entity_move) => {
                mapped_packet::MappedPacket::EntityMove(EntityMove {
                    entity_id: entity_move.entity_id,
                    delta_x: From::from(entity_move.delta_x),
                    delta_y: From::from(entity_move.delta_y),
                    delta_z: From::from(entity_move.delta_z),
                    on_ground: None,
                })
            }
            packet::Packet::EntityDestroy(destroy) => {
                mapped_packet::MappedPacket::EntityDestroy(EntityDestroy {
                    entity_ids: destroy.entity_ids.data.iter().map(|x| x.0).collect(),
                })
            }
            packet::Packet::EntityDestroy_u8(destroy) => {
                mapped_packet::MappedPacket::EntityDestroy(EntityDestroy {
                    entity_ids: destroy.entity_ids.data,
                })
            }
            packet::Packet::EditBook(edit_book) => {
                mapped_packet::MappedPacket::EditBook(EditBook {
                    new_book: edit_book.new_book,
                    is_signing: edit_book.is_signing,
                    hand: Hand::from(edit_book.hand.0),
                })
            }
            packet::Packet::Effect(effect) => mapped_packet::MappedPacket::Effect(Effect {
                effect_id: effect.effect_id,
                location: effect.location,
                data: effect.data,
                disable_relative: effect.disable_relative,
            }),
            packet::Packet::Effect_u8y(effect) => mapped_packet::MappedPacket::Effect(Effect {
                effect_id: effect.effect_id,
                location: Position::new(effect.x, effect.y as i32, effect.z),
                data: effect.data,
                disable_relative: effect.disable_relative,
            }),
            packet::Packet::EnchantItem(enchant_item) => {
                mapped_packet::MappedPacket::EnchantItem(EnchantItem {
                    id: enchant_item.id,
                    enchantment: enchant_item.enchantment,
                })
            }
            packet::Packet::EncryptionRequest(encryption_request) => {
                mapped_packet::MappedPacket::EncryptionRequest(EncryptionRequest {
                    server_id: encryption_request.server_id,
                    public_key: encryption_request.public_key.data,
                    verify_token: encryption_request.verify_token.data,
                })
            }
            packet::Packet::EncryptionRequest_i16(encryption_request) => {
                mapped_packet::MappedPacket::EncryptionRequest(EncryptionRequest {
                    server_id: encryption_request.server_id,
                    public_key: encryption_request.public_key.data,
                    verify_token: encryption_request.verify_token.data,
                })
            }
            packet::Packet::EncryptionResponse(encryption_response) => {
                mapped_packet::MappedPacket::EncryptionResponse(EncryptionResponse {
                    shared_secret: encryption_response.shared_secret.data,
                    verify_token: encryption_response.verify_token.data,
                })
            }
            packet::Packet::EncryptionResponse_i16(encryption_response) => {
                mapped_packet::MappedPacket::EncryptionResponse(EncryptionResponse {
                    shared_secret: encryption_response.shared_secret.data,
                    verify_token: encryption_response.verify_token.data,
                })
            }
            packet::Packet::EntityAction(action) => {
                mapped_packet::MappedPacket::EntityAction(EntityAction {
                    entity_id: action.entity_id,
                    action_id: action.action_id,
                })
            }
            packet::Packet::EntityAttach(attach) => {
                mapped_packet::MappedPacket::EntityAttach(EntityAttach {
                    entity_id: attach.entity_id,
                    vehicle: attach.vehicle,
                    leash: None,
                })
            }
            packet::Packet::EntityAttach_leashed(attach) => {
                mapped_packet::MappedPacket::EntityAttach(EntityAttach {
                    entity_id: attach.entity_id,
                    vehicle: attach.vehicle,
                    leash: Some(attach.leash),
                })
            }
            packet::Packet::EntityEffect(effect) => {
                mapped_packet::MappedPacket::EntityEffect(EntityEffect {
                    entity_id: effect.entity_id.0,
                    effect_id: effect.effect_id,
                    amplifier: effect.amplifier,
                    duration: effect.duration.0,
                    hide_particles: Some(effect.hide_particles),
                })
            }
            packet::Packet::EntityEffect_i32(effect) => {
                mapped_packet::MappedPacket::EntityEffect(EntityEffect {
                    entity_id: effect.entity_id,
                    effect_id: effect.effect_id,
                    amplifier: effect.amplifier,
                    duration: effect.duration as i32,
                    hide_particles: None,
                })
            }
            packet::Packet::EntityEquipment_Array(equipment) => {
                mapped_packet::MappedPacket::EntityEquipment_Array(EntityEquipment_Array {
                    entity_id: equipment.entity_id.0,
                    equipments: equipment.equipments,
                })
            }
            packet::Packet::EntityEquipment_u16(equipment) => {
                mapped_packet::MappedPacket::EntityEquipment_Single(EntityEquipment_Single {
                    entity_id: equipment.entity_id.0,
                    slot: equipment.slot as i32,
                    item: equipment.item,
                })
            }
            packet::Packet::EntityEquipment_u16_i32(equipment) => {
                mapped_packet::MappedPacket::EntityEquipment_Single(EntityEquipment_Single {
                    entity_id: equipment.entity_id,
                    slot: equipment.slot as i32,
                    item: equipment.item,
                })
            }
            packet::Packet::EntityEquipment_VarInt(equipment) => {
                mapped_packet::MappedPacket::EntityEquipment_Single(EntityEquipment_Single {
                    entity_id: equipment.entity_id.0,
                    slot: equipment.slot.0,
                    item: equipment.item,
                })
            }
            packet::Packet::EntityMetadata(metadata) => {
                mapped_packet::MappedPacket::EntityMetadata(EntityMetadata {
                    entity_id: metadata.entity_id.0,
                    metadata: metadata.metadata,
                })
            }
            packet::Packet::EntityMetadata_i32(metadata) => {
                mapped_packet::MappedPacket::EntityMetadata(EntityMetadata {
                    entity_id: metadata.entity_id,
                    metadata: metadata.metadata,
                })
            }
            packet::Packet::EntityProperties(properties) => {
                mapped_packet::MappedPacket::EntityProperties(EntityProperties {
                    entity_id: properties.entity_id.0,
                    properties: properties
                        .properties
                        .data
                        .into_iter()
                        .map(|x| EntityProperty {
                            key: x.key,
                            value: x.value,
                            modifiers: x.modifiers.data,
                        })
                        .collect(),
                })
            }
            packet::Packet::EntityProperties_i32(properties) => {
                mapped_packet::MappedPacket::EntityProperties(EntityProperties {
                    entity_id: properties.entity_id,
                    properties: properties
                        .properties
                        .data
                        .into_iter()
                        .map(|x| EntityProperty {
                            key: x.key,
                            value: x.value,
                            modifiers: x.modifiers.data,
                        })
                        .collect(),
                })
            }
            packet::Packet::EntityRemoveEffect(remove_effect) => {
                mapped_packet::MappedPacket::EntityRemoveEffect(EntityRemoveEffect {
                    entity_id: remove_effect.entity_id.0,
                    effect_id: remove_effect.effect_id,
                })
            }
            packet::Packet::EntityRemoveEffect_i32(remove_effect) => {
                mapped_packet::MappedPacket::EntityRemoveEffect(EntityRemoveEffect {
                    entity_id: remove_effect.entity_id,
                    effect_id: remove_effect.effect_id,
                })
            }
            packet::Packet::EntitySoundEffect(sound_effect) => {
                mapped_packet::MappedPacket::EntitySoundEffect(EntitySoundEffect {
                    sound_id: sound_effect.sound_id.0,
                    sound_category: sound_effect.sound_category.0,
                    entity_id: sound_effect.entity_id.0,
                    volume: sound_effect.volume,
                    pitch: sound_effect.pitch,
                })
            }
            packet::Packet::EntityStatus(status) => {
                mapped_packet::MappedPacket::EntityStatus(EntityStatus {
                    entity_id: status.entity_id,
                    entity_status: status.entity_status,
                })
            }
            packet::Packet::EntityUpdateNBT(update_nbt) => {
                mapped_packet::MappedPacket::EntityUpdateNBT(EntityUpdateNBT {
                    entity_id: update_nbt.entity_id.0,
                    nbt: update_nbt.nbt,
                })
            }
            packet::Packet::EntityUsedBed(used_bed) => {
                mapped_packet::MappedPacket::EntityUsedBed(EntityUsedBed {
                    entity_id: used_bed.entity_id.0,
                    location: used_bed.location,
                })
            }
            packet::Packet::EntityUsedBed_i32(used_bed) => {
                mapped_packet::MappedPacket::EntityUsedBed(EntityUsedBed {
                    entity_id: used_bed.entity_id,
                    location: Position::new(used_bed.x, used_bed.y as i32, used_bed.z),
                })
            }
            packet::Packet::Explosion(explosion) => {
                mapped_packet::MappedPacket::Explosion(Explosion {
                    x: explosion.x,
                    y: explosion.y,
                    z: explosion.z,
                    radius: explosion.radius,
                    records: explosion.records.data,
                    velocity_x: explosion.velocity_x,
                    velocity_y: explosion.velocity_y,
                    velocity_z: explosion.velocity_z,
                })
            }
            packet::Packet::FacePlayer(face_player) => {
                mapped_packet::MappedPacket::FacePlayer(FacePlayer {
                    feet_eyes: face_player.feet_eyes.0,
                    target_x: face_player.target_x,
                    target_y: face_player.target_y,
                    target_z: face_player.target_z,
                    is_entity: face_player.is_entity,
                    entity_id: face_player.entity_id.map(|x| x.0),
                    entity_feet_eyes: face_player.entity_feet_eyes.map(|x| x.0),
                })
            }
            packet::Packet::GenerateStructure(generate_structure) => {
                mapped_packet::MappedPacket::GenerateStructure(GenerateStructure {
                    location: generate_structure.location,
                    levels: generate_structure.levels.0,
                    keep_jigsaws: generate_structure.keep_jigsaws,
                })
            }
            packet::Packet::HeldItemChange(held_item) => {
                mapped_packet::MappedPacket::HeldItemChange(HeldItemChange {
                    slot: held_item.slot,
                })
            }
            packet::Packet::Handshake(handshake) => {
                mapped_packet::MappedPacket::Handshake(Handshake {
                    protocol_version: handshake.protocol_version.0,
                    host: handshake.host,
                    port: handshake.port,
                    next: handshake.next.0,
                })
            }
            packet::Packet::JoinGame_i8(join_game) => {
                mapped_packet::MappedPacket::JoinGame_i8(JoinGame_i8 {
                    entity_id: join_game.entity_id,
                    gamemode: join_game.gamemode,
                    dimension: join_game.dimension,
                    difficulty: join_game.difficulty,
                    max_players: join_game.max_players,
                    level_type: join_game.level_type,
                    reduced_debug_info: join_game.reduced_debug_info,
                })
            }
            packet::Packet::JoinGame_i8_NoDebug(join_game) => {
                mapped_packet::MappedPacket::JoinGame_i8_NoDebug(JoinGame_i8_NoDebug {
                    entity_id: join_game.entity_id,
                    gamemode: join_game.gamemode,
                    dimension: join_game.dimension,
                    difficulty: join_game.difficulty,
                    max_players: join_game.max_players,
                    level_type: join_game.level_type,
                })
            }
            packet::Packet::JoinGame_i32(join_game) => {
                mapped_packet::MappedPacket::JoinGame_i32(JoinGame_i32 {
                    entity_id: join_game.entity_id,
                    gamemode: join_game.gamemode,
                    dimension: join_game.dimension,
                    difficulty: join_game.difficulty,
                    max_players: join_game.max_players,
                    level_type: join_game.level_type,
                    reduced_debug_info: join_game.reduced_debug_info,
                })
            }
            packet::Packet::JoinGame_i32_ViewDistance(join_game) => {
                mapped_packet::MappedPacket::JoinGame_i32_ViewDistance(JoinGame_i32_ViewDistance {
                    entity_id: join_game.entity_id,
                    gamemode: join_game.gamemode,
                    dimension: join_game.dimension,
                    max_players: join_game.max_players,
                    level_type: join_game.level_type,
                    view_distance: join_game.view_distance.0,
                    reduced_debug_info: join_game.reduced_debug_info,
                })
            }
            packet::Packet::JoinGame_WorldNames(join_game) => {
                mapped_packet::MappedPacket::JoinGame_WorldNames(JoinGame_WorldNames {
                    entity_id: join_game.entity_id,
                    gamemode: join_game.gamemode,
                    previous_gamemode: join_game.previous_gamemode,
                    world_names: join_game.world_names.data,
                    dimension_codec: join_game.dimension_codec,
                    dimension: join_game.dimension,
                    world_name: join_game.world_name,
                    hashed_seed: join_game.hashed_seed,
                    max_players: join_game.max_players,
                    view_distance: join_game.view_distance.0,
                    reduced_debug_info: join_game.reduced_debug_info,
                    enable_respawn_screen: join_game.enable_respawn_screen,
                    is_debug: join_game.is_debug,
                    is_flat: join_game.is_flat,
                })
            }
            packet::Packet::JoinGame_WorldNames_IsHard(join_game) => {
                mapped_packet::MappedPacket::JoinGame(
                    JoinGame {
                        entity_id: join_game.entity_id,
                        is_hardcore: Some(join_game.is_hardcore),
                        gamemode: join_game.gamemode,
                        previous_gamemode: Some(join_game.previous_gamemode),
                        world_names: Some(join_game.world_names.data),
                        dimension_codec: Some(join_game.dimension_codec),
                        dimension: Some(join_game.dimension),
                        dimension_id: None,
                        difficulty: None,
                        level_type: None,
                        world_name: Some(join_game.world_name),
                        hashed_seed: Some(join_game.hashed_seed),
                        max_players: join_game.max_players.0,
                        view_distance: join_game.view_distance.0,
                        reduced_debug_info: Some(join_game.reduced_debug_info),
                        enable_respawn_screen: Some(join_game.enable_respawn_screen),
                        is_debug: Some(join_game.is_debug),
                        is_flat: Some(join_game.is_flat),
                    },
                )
            }
            packet::Packet::JoinGame_HashedSeed_Respawn(join_game) => {
                mapped_packet::MappedPacket::JoinGame(
                    JoinGame {
                        entity_id: join_game.entity_id,
                        is_hardcore: None,
                        gamemode: join_game.gamemode,
                        previous_gamemode: None,
                        world_names: None,
                        dimension_codec: None,
                        dimension: Some(join_game.dimension),
                        dimension_id: None,
                        difficulty: None,
                        level_type: Some(join_game.level_type),
                        world_name: None,
                        hashed_seed: Some(join_game.hashed_seed),
                        max_players: join_game.max_players as i32,
                        view_distance: join_game.view_distance.0,
                        reduced_debug_info: Some(join_game.reduced_debug_info),
                        enable_respawn_screen: Some(join_game.enable_respawn_screen),
                        is_debug: None,
                        is_flat: None,
                    },
                )
            }
            packet::Packet::KeepAliveClientbound_i32(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveClientbound(KeepAliveClientbound {
                    id: keep_alive.id as i64,
                })
            }
            packet::Packet::KeepAliveClientbound_i64(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveClientbound(KeepAliveClientbound {
                    id: keep_alive.id,
                })
            }
            packet::Packet::KeepAliveClientbound_VarInt(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveClientbound(KeepAliveClientbound {
                    id: keep_alive.id.0 as i64,
                })
            }
            packet::Packet::KeepAliveServerbound_i32(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveServerbound(KeepAliveServerbound {
                    id: keep_alive.id as i64,
                })
            }
            packet::Packet::KeepAliveServerbound_i64(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveServerbound(KeepAliveServerbound {
                    id: keep_alive.id,
                })
            }
            packet::Packet::KeepAliveServerbound_VarInt(keep_alive) => {
                mapped_packet::MappedPacket::KeepAliveServerbound(KeepAliveServerbound {
                    id: keep_alive.id.0 as i64,
                })
            }
            packet::Packet::LockDifficulty(lock_difficulty) => {
                mapped_packet::MappedPacket::LockDifficulty(LockDifficulty {
                    locked: lock_difficulty.locked,
                })
            }
            packet::Packet::LoginDisconnect(login_disconnect) => {
                mapped_packet::MappedPacket::LoginDisconnect(LoginDisconnect {
                    reason: login_disconnect.reason,
                })
            }
            packet::Packet::LoginPluginRequest(plugin_request) => {
                mapped_packet::MappedPacket::LoginPluginRequest(LoginPluginRequest {
                    message_id: plugin_request.message_id.0,
                    channel: plugin_request.channel,
                    data: plugin_request.data,
                })
            }
            packet::Packet::LoginPluginResponse(plugin_response) => {
                mapped_packet::MappedPacket::LoginPluginResponse(LoginPluginResponse {
                    message_id: plugin_response.message_id.0,
                    successful: plugin_response.successful,
                    data: plugin_response.data,
                })
            }
            packet::Packet::LoginStart(login_start) => {
                mapped_packet::MappedPacket::LoginStart(LoginStart {
                    username: login_start.username,
                })
            }
            packet::Packet::LoginSuccess_String(login_success) => {
                mapped_packet::MappedPacket::LoginSuccess_String(LoginSuccess_String {
                    uuid: login_success.uuid,
                    username: login_success.username,
                })
            }
            packet::Packet::LoginSuccess_UUID(login_success) => {
                mapped_packet::MappedPacket::LoginSuccess_UUID(LoginSuccess_UUID {
                    uuid: login_success.uuid,
                    username: login_success.username,
                })
            }
            packet::Packet::Maps(maps) => mapped_packet::MappedPacket::Maps(Maps {
                item_damage: maps.item_damage.0,
                scale: Some(maps.scale),
                tracking_position: Some(maps.tracking_position),
                locked: Some(maps.locked),
                icons: Some(maps.icons.data),
                columns: Some(maps.columns),
                rows: maps.rows,
                x: maps.x,
                z: maps.z,
                data: maps.data.map(|x| x.data),
            }),
            packet::Packet::Maps_NoLocked(maps) => mapped_packet::MappedPacket::Maps(Maps {
                item_damage: maps.item_damage.0,
                scale: Some(maps.scale),
                tracking_position: Some(maps.tracking_position),
                locked: None,
                icons: Some(maps.icons.data),
                columns: Some(maps.columns),
                rows: maps.rows,
                x: maps.x,
                z: maps.z,
                data: maps.data.map(|x| x.data),
            }),
            packet::Packet::Maps_NoTracking(maps) => mapped_packet::MappedPacket::Maps(Maps {
                item_damage: maps.item_damage.0,
                scale: Some(maps.scale),
                tracking_position: None,
                locked: None,
                icons: Some(maps.icons.data),
                columns: Some(maps.columns),
                rows: maps.rows,
                x: maps.x,
                z: maps.z,
                data: maps.data.map(|x| x.data),
            }),
            packet::Packet::Maps_NoTracking_Data(maps) => mapped_packet::MappedPacket::Maps(Maps {
                item_damage: maps.item_damage.0,
                scale: None,
                tracking_position: None,
                locked: None,
                icons: None,
                columns: None,
                rows: None,
                x: None,
                z: None,
                data: None,
            }),
            packet::Packet::MultiBlockChange_Packed(block_change) => {
                let sx = (block_change.chunk_section_pos >> 42) as i32;
                let sy = ((block_change.chunk_section_pos << 44) >> 44) as i32;
                let sz = ((block_change.chunk_section_pos << 22) >> 42) as i32;
                mapped_packet::MappedPacket::MultiBlockChange(MultiBlockChange {
                    chunk_x: sx,
                    chunk_y: Some(sy),
                    chunk_z: sz,
                    no_trust_edges: Some(block_change.no_trust_edges),
                    records: block_change
                        .records
                        .data
                        .iter()
                        .map(|record| {
                            let block_id = record.0 >> 12;
                            let z = (record.0 & 0xf) as u8;
                            let y = ((record.0 >> 4) & 0xf) as u8;
                            let x = ((record.0 >> 8) & 0xf) as u8;
                            let xz = (z & 0xF) | (x << 4);
                            BlockChangeRecord {
                                xz,
                                y,
                                block_id: block_id as i32,
                            }
                        })
                        .collect(),
                })
            }
            packet::Packet::MultiBlockChange_u16(block_change) => {
                let mut cursor = Cursor::new(block_change.data);
                let mut records = vec![];
                for _ in 0..block_change.record_count {
                    let record = cursor.read_u32::<BigEndian>().unwrap();

                    let id = record & 0x0000_ffff;
                    let y = ((record & 0x00ff_0000) >> 16) as u8;
                    let z = ((record & 0x0f00_0000) >> 24) as u8;
                    let x = ((record & 0xf000_0000) >> 28) as u8;
                    let xz = (z & 0xF) | (x << 4);
                    records.push(BlockChangeRecord {
                        xz,
                        y,
                        block_id: id as i32,
                    });
                }
                mapped_packet::MappedPacket::MultiBlockChange(MultiBlockChange {
                    chunk_x: block_change.chunk_x,
                    chunk_y: None,
                    chunk_z: block_change.chunk_z,
                    no_trust_edges: None,
                    records,
                })
            }
            packet::Packet::MultiBlockChange_VarInt(block_change) => {
                mapped_packet::MappedPacket::MultiBlockChange(MultiBlockChange {
                    chunk_x: block_change.chunk_x,
                    chunk_y: None,
                    chunk_z: block_change.chunk_z,
                    no_trust_edges: None,
                    records: block_change
                        .records
                        .data
                        .iter()
                        .map(|record| BlockChangeRecord {
                            xz: record.xz,
                            y: record.y,
                            block_id: record.block_id.0,
                        })
                        .collect(),
                })
            }
            packet::Packet::NamedSoundEffect(sound_effect) => {
                mapped_packet::MappedPacket::NamedSoundEffect(NamedSoundEffect {
                    name: sound_effect.name,
                    category: Some(sound_effect.category.0),
                    x: sound_effect.x,
                    y: sound_effect.y,
                    z: sound_effect.z,
                    volume: sound_effect.volume,
                    pitch: sound_effect.pitch,
                })
            }
            packet::Packet::NamedSoundEffect_u8(sound_effect) => {
                mapped_packet::MappedPacket::NamedSoundEffect(NamedSoundEffect {
                    name: sound_effect.name,
                    category: Some(sound_effect.category.0),
                    x: sound_effect.x,
                    y: sound_effect.y,
                    z: sound_effect.z,
                    volume: sound_effect.volume,
                    pitch: sound_effect.pitch as f32, // TODO: Conversion?
                })
            }
            packet::Packet::NamedSoundEffect_u8_NoCategory(sound_effect) => {
                mapped_packet::MappedPacket::NamedSoundEffect(NamedSoundEffect {
                    name: sound_effect.name,
                    category: None,
                    x: sound_effect.x,
                    y: sound_effect.y,
                    z: sound_effect.z,
                    volume: sound_effect.volume,
                    pitch: sound_effect.pitch as f32, // TODO: Conversion?
                })
            }
            packet::Packet::NameItem(name_item) => {
                mapped_packet::MappedPacket::NameItem(NameItem {
                    item_name: name_item.item_name,
                })
            }
            packet::Packet::NBTQueryResponse(nbt_query) => {
                mapped_packet::MappedPacket::NBTQueryResponse(NBTQueryResponse {
                    transaction_id: nbt_query.transaction_id.0,
                    nbt: nbt_query.nbt,
                })
            }
            packet::Packet::OpenBook(open_book) => {
                mapped_packet::MappedPacket::OpenBook(OpenBook {
                    hand: Hand::from(open_book.hand.0),
                })
            }
            packet::Packet::Player(player) => mapped_packet::MappedPacket::Player(Player {
                on_ground: player.on_ground,
            }),
            packet::Packet::PlayerDigging(digging) => {
                mapped_packet::MappedPacket::PlayerDigging(PlayerDigging {
                    status: digging.status.0,
                    location: digging.location,
                    face: digging.face,
                })
            }
            packet::Packet::PlayerDigging_u8(digging) => {
                mapped_packet::MappedPacket::PlayerDigging(PlayerDigging {
                    status: digging.status as i32,
                    location: digging.location,
                    face: digging.face,
                })
            }
            packet::Packet::PlayerDigging_u8_u8y(digging) => {
                mapped_packet::MappedPacket::PlayerDigging(PlayerDigging {
                    status: digging.status as i32,
                    location: Position::new(digging.x, digging.y as i32, digging.z),
                    face: digging.face,
                })
            }
            packet::Packet::PlayerInfo_String(info) => {
                mapped_packet::MappedPacket::PlayerInfo_String(PlayerInfo_String {
                    name: info.name,
                    online: info.online,
                    ping: info.ping,
                })
            }
            packet::Packet::PlayerInfo(info) => {
                mapped_packet::MappedPacket::PlayerInfo(PlayerInfo { inner: info.inner })
            }
            packet::Packet::Particle_Data(particle) => {
                mapped_packet::MappedPacket::Particle(Particle {
                    particle_id: Some(particle.particle_id),
                    particle_name: None,
                    long_distance: Some(particle.long_distance),
                    x: particle.x as f64,
                    y: particle.y as f64,
                    z: particle.z as f64,
                    offset_x: particle.offset_x,
                    offset_y: particle.offset_y,
                    offset_z: particle.offset_z,
                    speed: particle.speed,
                    count: particle.count,
                    block_state: Some(particle.block_state.0),
                    red: Some(particle.red),
                    green: Some(particle.green),
                    blue: Some(particle.blue),
                    scale: Some(particle.scale),
                    item: particle.item,
                    data1: None,
                    data2: None,
                })
            }
            packet::Packet::Particle_Data13(particle) => {
                mapped_packet::MappedPacket::Particle(Particle {
                    particle_id: Some(particle.particle_id),
                    particle_name: None,
                    long_distance: Some(particle.long_distance),
                    x: particle.x as f64,
                    y: particle.y as f64,
                    z: particle.z as f64,
                    offset_x: particle.offset_x,
                    offset_y: particle.offset_y,
                    offset_z: particle.offset_z,
                    speed: particle.speed,
                    count: particle.count,
                    block_state: Some(particle.block_state.0),
                    red: Some(particle.red),
                    green: Some(particle.green),
                    blue: Some(particle.blue),
                    scale: Some(particle.scale),
                    item: particle.item,
                    data1: None,
                    data2: None,
                })
            }
            packet::Packet::Particle_f64(particle) => {
                mapped_packet::MappedPacket::Particle(Particle {
                    particle_id: Some(particle.particle_id),
                    particle_name: None,
                    long_distance: Some(particle.long_distance),
                    x: particle.x,
                    y: particle.y,
                    z: particle.z,
                    offset_x: particle.offset_x,
                    offset_y: particle.offset_y,
                    offset_z: particle.offset_z,
                    speed: particle.speed,
                    count: particle.count,
                    block_state: Some(particle.block_state.0),
                    red: Some(particle.red),
                    green: Some(particle.green),
                    blue: Some(particle.blue),
                    scale: Some(particle.scale),
                    item: particle.item,
                    data1: None,
                    data2: None,
                })
            }
            packet::Packet::Particle_Named(particle) => {
                mapped_packet::MappedPacket::Particle(Particle {
                    particle_id: None,
                    particle_name: Some(particle.particle_id),
                    long_distance: None,
                    x: particle.x as f64,
                    y: particle.y as f64,
                    z: particle.z as f64,
                    offset_x: particle.offset_x,
                    offset_y: particle.offset_y,
                    offset_z: particle.offset_z,
                    speed: particle.speed,
                    count: particle.count,
                    block_state: None,
                    red: None,
                    green: None,
                    blue: None,
                    scale: None,
                    item: None,
                    data1: None,
                    data2: None,
                })
            }
            packet::Packet::Particle_VarIntArray(particle) => {
                mapped_packet::MappedPacket::Particle(Particle {
                    particle_id: Some(particle.particle_id),
                    particle_name: None,
                    long_distance: Some(particle.long_distance),
                    x: particle.x as f64,
                    y: particle.y as f64,
                    z: particle.z as f64,
                    offset_x: particle.offset_x,
                    offset_y: particle.offset_y,
                    offset_z: particle.offset_z,
                    speed: particle.speed,
                    count: particle.count,
                    block_state: None,
                    red: None,
                    green: None,
                    blue: None,
                    scale: None,
                    item: None,
                    data1: Some(particle.data1.0),
                    data2: Some(particle.data2.0),
                })
            }
            packet::Packet::PickItem(pick_item) => {
                mapped_packet::MappedPacket::PickItem(PickItem {
                    slot_to_use: pick_item.slot_to_use.0,
                })
            }
            packet::Packet::PlayerAbilities(abilities) => {
                mapped_packet::MappedPacket::PlayerAbilities(PlayerAbilities {
                    flags: abilities.flags,
                    flying_speed: abilities.flying_speed,
                    walking_speed: abilities.walking_speed,
                })
            }
            packet::Packet::PlayerAction(action) => {
                mapped_packet::MappedPacket::PlayerAction(PlayerAction {
                    entity_id: action.entity_id.0,
                    action_id: action.action_id.0,
                    jump_boost: action.jump_boost.0,
                })
            }
            packet::Packet::PlayerAction_i32(action) => {
                mapped_packet::MappedPacket::PlayerAction(PlayerAction {
                    entity_id: action.entity_id,
                    action_id: action.action_id as i32,
                    jump_boost: action.jump_boost,
                })
            }
            packet::Packet::PlayerBlockPlacement_f32(block_placement) => {
                mapped_packet::MappedPacket::PlayerBlockPlacement(PlayerBlockPlacement {
                    location: block_placement.location,
                    face: block_placement.face.0,
                    hand: Some(block_placement.hand.0),
                    hand_item: None,
                    cursor_x: block_placement.cursor_x,
                    cursor_y: block_placement.cursor_y,
                    cursor_z: block_placement.cursor_z,
                    inside_block: None,
                })
            }
            packet::Packet::PlayerBlockPlacement_insideblock(block_placement) => {
                mapped_packet::MappedPacket::PlayerBlockPlacement(PlayerBlockPlacement {
                    location: block_placement.location,
                    face: block_placement.face.0,
                    hand: Some(block_placement.hand.0),
                    hand_item: None,
                    cursor_x: block_placement.cursor_x,
                    cursor_y: block_placement.cursor_y,
                    cursor_z: block_placement.cursor_z,
                    inside_block: Some(block_placement.inside_block),
                })
            }
            packet::Packet::PlayerBlockPlacement_u8(block_placement) => {
                mapped_packet::MappedPacket::PlayerBlockPlacement(PlayerBlockPlacement {
                    location: block_placement.location,
                    face: block_placement.face.0,
                    hand: Some(block_placement.hand.0),
                    hand_item: None,
                    cursor_x: block_placement.cursor_x as f32, // TODO: Map this properly!
                    cursor_y: block_placement.cursor_y as f32, // TODO: Map this properly!
                    cursor_z: block_placement.cursor_z as f32, // TODO: Map this properly!
                    inside_block: None,
                })
            }
            packet::Packet::PlayerBlockPlacement_u8_Item(block_placement) => {
                mapped_packet::MappedPacket::PlayerBlockPlacement(PlayerBlockPlacement {
                    location: block_placement.location,
                    face: block_placement.face as i32,
                    hand: None,
                    hand_item: block_placement.hand,
                    cursor_x: block_placement.cursor_x as f32, // TODO: Map this properly!
                    cursor_y: block_placement.cursor_y as f32, // TODO: Map this properly!
                    cursor_z: block_placement.cursor_z as f32, // TODO: Map this properly!
                    inside_block: None,
                })
            }
            packet::Packet::PlayerBlockPlacement_u8_Item_u8y(block_placement) => {
                mapped_packet::MappedPacket::PlayerBlockPlacement(PlayerBlockPlacement {
                    location: Position::new(
                        block_placement.x,
                        block_placement.y as i32,
                        block_placement.z,
                    ),
                    face: block_placement.face as i32,
                    hand: None,
                    hand_item: block_placement.hand,
                    cursor_x: block_placement.cursor_x as f32, // TODO: Map this properly!
                    cursor_y: block_placement.cursor_y as f32, // TODO: Map this properly!
                    cursor_z: block_placement.cursor_z as f32, // TODO: Map this properly!
                    inside_block: None,
                })
            }
            packet::Packet::PlayerListHeaderFooter(list_header_footer) => {
                mapped_packet::MappedPacket::PlayerListHeaderFooter(PlayerListHeaderFooter {
                    header: list_header_footer.header,
                    footer: list_header_footer.footer,
                })
            }
            packet::Packet::PlayerLook(look) => {
                mapped_packet::MappedPacket::PlayerLook(PlayerLook {
                    yaw: look.yaw,
                    pitch: look.pitch,
                    on_ground: look.on_ground,
                })
            }
            packet::Packet::PlayerPosition(position) => {
                mapped_packet::MappedPacket::PlayerPosition(PlayerPosition {
                    x: position.x,
                    y: Some(position.y),
                    z: position.z,
                    feet_y: None,
                    head_y: None,
                    on_ground: position.on_ground,
                })
            }
            packet::Packet::PlayerPosition_HeadY(position) => {
                mapped_packet::MappedPacket::PlayerPosition(PlayerPosition {
                    x: position.x,
                    y: None,
                    z: position.z,
                    feet_y: Some(position.feet_y),
                    head_y: Some(position.head_y),
                    on_ground: position.on_ground,
                })
            }
            packet::Packet::PlayerPositionLook(position_look) => {
                mapped_packet::MappedPacket::PlayerPositionLook(PlayerPositionLook {
                    x: position_look.x,
                    y: Some(position_look.y),
                    z: position_look.z,
                    feet_y: None,
                    head_y: None,
                    yaw: position_look.yaw,
                    pitch: position_look.pitch,
                    on_ground: position_look.on_ground,
                })
            }
            packet::Packet::PlayerPositionLook_HeadY(position_look) => {
                mapped_packet::MappedPacket::PlayerPositionLook(PlayerPositionLook {
                    x: position_look.x,
                    y: None,
                    z: position_look.z,
                    feet_y: Some(position_look.feet_y),
                    head_y: Some(position_look.head_y),
                    yaw: position_look.yaw,
                    pitch: position_look.pitch,
                    on_ground: position_look.on_ground,
                })
            }
            packet::Packet::PluginMessageClientbound(plugin_msg) => {
                mapped_packet::MappedPacket::PluginMessageClientbound(PluginMessageClientbound {
                    channel: plugin_msg.channel,
                    data: plugin_msg.data,
                })
            }
            packet::Packet::PluginMessageClientbound_i16(plugin_msg) => {
                mapped_packet::MappedPacket::PluginMessageClientbound(PluginMessageClientbound {
                    channel: plugin_msg.channel,
                    data: plugin_msg.data.data,
                })
            }
            packet::Packet::PluginMessageServerbound(plugin_msg) => {
                mapped_packet::MappedPacket::PluginMessageServerbound(PluginMessageServerbound {
                    channel: plugin_msg.channel,
                    data: plugin_msg.data,
                })
            }
            packet::Packet::PluginMessageServerbound_i16(plugin_msg) => {
                mapped_packet::MappedPacket::PluginMessageServerbound(PluginMessageServerbound {
                    channel: plugin_msg.channel,
                    data: plugin_msg.data.data,
                })
            }
            packet::Packet::QueryBlockNBT(block_nbt) => {
                mapped_packet::MappedPacket::QueryBlockNBT(QueryBlockNBT {
                    transaction_id: block_nbt.transaction_id.0,
                    location: block_nbt.location,
                })
            }
            packet::Packet::QueryEntityNBT(entity_nbt) => {
                mapped_packet::MappedPacket::QueryEntityNBT(QueryEntityNBT {
                    transaction_id: entity_nbt.transaction_id.0,
                    entity_id: entity_nbt.entity_id.0,
                })
            }
            packet::Packet::Respawn_WorldName(respawn) => {
                mapped_packet::MappedPacket::Respawn(Respawn {
                    dimension_tag: None,
                    dimension_name: Some(respawn.dimension),
                    world_name: Some(respawn.world_name),
                    dimension: None,
                    hashed_seed: Some(respawn.hashed_seed),
                    difficulty: None,
                    gamemode: respawn.gamemode,
                    level_type: None,
                    previous_gamemode: Some(respawn.previous_gamemode),
                    is_debug: Some(respawn.is_debug),
                    is_flat: Some(respawn.is_flat),
                    copy_metadata: Some(respawn.copy_metadata),
                })
            }
            packet::Packet::Respawn_NBT(respawn) => mapped_packet::MappedPacket::Respawn(Respawn {
                dimension_tag: respawn.dimension,
                dimension_name: None,
                world_name: Some(respawn.world_name),
                dimension: None,
                hashed_seed: Some(respawn.hashed_seed),
                difficulty: None,
                gamemode: respawn.gamemode,
                level_type: None,
                previous_gamemode: Some(respawn.previous_gamemode),
                is_debug: Some(respawn.is_debug),
                is_flat: Some(respawn.is_flat),
                copy_metadata: Some(respawn.copy_metadata),
            }),
            packet::Packet::Respawn_HashedSeed(respawn) => {
                mapped_packet::MappedPacket::Respawn(Respawn {
                    dimension_tag: None,
                    dimension_name: None,
                    world_name: None,
                    dimension: Some(respawn.dimension),
                    hashed_seed: Some(respawn.hashed_seed),
                    difficulty: Some(respawn.difficulty),
                    gamemode: respawn.gamemode,
                    level_type: Some(respawn.level_type),
                    previous_gamemode: None,
                    is_debug: None,
                    is_flat: None,
                    copy_metadata: None,
                })
            }
            packet::Packet::Respawn_Gamemode(respawn) => {
                mapped_packet::MappedPacket::Respawn(Respawn {
                    dimension_tag: None,
                    dimension_name: None,
                    world_name: None,
                    dimension: Some(respawn.dimension),
                    hashed_seed: None,
                    difficulty: Some(respawn.difficulty),
                    gamemode: respawn.gamemode,
                    level_type: Some(respawn.level_type),
                    previous_gamemode: None,
                    is_debug: None,
                    is_flat: None,
                    copy_metadata: None,
                })
            }
            packet::Packet::ResourcePackSend(resource_pack) => {
                mapped_packet::MappedPacket::ResourcePackSend(ResourcePackSend {
                    url: resource_pack.url,
                    hash: resource_pack.hash,
                })
            }
            packet::Packet::ResourcePackStatus(resource_pack) => {
                mapped_packet::MappedPacket::ResourcePackStatus(ResourcePackStatus {
                    hash: None,
                    result: resource_pack.result.0,
                })
            }
            packet::Packet::ResourcePackStatus_hash(resource_pack) => {
                mapped_packet::MappedPacket::ResourcePackStatus(ResourcePackStatus {
                    hash: Some(resource_pack.hash),
                    result: resource_pack.result.0,
                })
            }
            packet::Packet::SpawnMob_WithMeta(spawn_mob) => {
                mapped_packet::MappedPacket::SpawnMob(SpawnMob {
                    entity_id: spawn_mob.entity_id.0,
                    uuid: Some(spawn_mob.uuid),
                    ty: spawn_mob.ty.0,
                    x: spawn_mob.x,
                    y: spawn_mob.y,
                    z: spawn_mob.z,
                    yaw: spawn_mob.yaw,
                    pitch: spawn_mob.pitch,
                    head_pitch: spawn_mob.head_pitch,
                    velocity_x: spawn_mob.velocity_x,
                    velocity_y: spawn_mob.velocity_y,
                    velocity_z: spawn_mob.velocity_z,
                    metadata: Some(spawn_mob.metadata),
                })
            }
            packet::Packet::SpawnMob_NoMeta(spawn_mob) => {
                mapped_packet::MappedPacket::SpawnMob(SpawnMob {
                    entity_id: spawn_mob.entity_id.0,
                    uuid: Some(spawn_mob.uuid),
                    ty: spawn_mob.ty.0,
                    x: spawn_mob.x,
                    y: spawn_mob.y,
                    z: spawn_mob.z,
                    yaw: spawn_mob.yaw,
                    pitch: spawn_mob.pitch,
                    head_pitch: spawn_mob.head_pitch,
                    velocity_x: spawn_mob.velocity_x,
                    velocity_y: spawn_mob.velocity_y,
                    velocity_z: spawn_mob.velocity_z,
                    metadata: None,
                })
            }
            packet::Packet::SpawnMob_u8(spawn_mob) => {
                mapped_packet::MappedPacket::SpawnMob(SpawnMob {
                    entity_id: spawn_mob.entity_id.0,
                    uuid: Some(spawn_mob.uuid),
                    ty: spawn_mob.ty as i32,
                    x: spawn_mob.x,
                    y: spawn_mob.y,
                    z: spawn_mob.z,
                    yaw: spawn_mob.yaw,
                    pitch: spawn_mob.pitch,
                    head_pitch: spawn_mob.head_pitch,
                    velocity_x: spawn_mob.velocity_x,
                    velocity_y: spawn_mob.velocity_y,
                    velocity_z: spawn_mob.velocity_z,
                    metadata: Some(spawn_mob.metadata),
                })
            }
            packet::Packet::SpawnMob_u8_i32(spawn_mob) => {
                mapped_packet::MappedPacket::SpawnMob(SpawnMob {
                    entity_id: spawn_mob.entity_id.0,
                    uuid: Some(spawn_mob.uuid),
                    ty: spawn_mob.ty as i32,
                    x: From::from(spawn_mob.x),
                    y: From::from(spawn_mob.y),
                    z: From::from(spawn_mob.z),
                    yaw: spawn_mob.yaw,
                    pitch: spawn_mob.pitch,
                    head_pitch: spawn_mob.head_pitch,
                    velocity_x: spawn_mob.velocity_x,
                    velocity_y: spawn_mob.velocity_y,
                    velocity_z: spawn_mob.velocity_z,
                    metadata: Some(spawn_mob.metadata),
                })
            }
            packet::Packet::SpawnMob_u8_i32_NoUUID(spawn_mob) => {
                mapped_packet::MappedPacket::SpawnMob(SpawnMob {
                    entity_id: spawn_mob.entity_id.0,
                    uuid: None,
                    ty: spawn_mob.ty as i32,
                    x: From::from(spawn_mob.x),
                    y: From::from(spawn_mob.y),
                    z: From::from(spawn_mob.z),
                    yaw: spawn_mob.yaw,
                    pitch: spawn_mob.pitch,
                    head_pitch: spawn_mob.head_pitch,
                    velocity_x: spawn_mob.velocity_x,
                    velocity_y: spawn_mob.velocity_y,
                    velocity_z: spawn_mob.velocity_z,
                    metadata: Some(spawn_mob.metadata),
                })
            }
            packet::Packet::SpawnObject(spawn_object) => {
                mapped_packet::MappedPacket::SpawnObject(SpawnObject {
                    entity_id: spawn_object.entity_id.0,
                    uuid: Some(spawn_object.uuid),
                    ty: spawn_object.ty as i32,
                    x: spawn_object.x,
                    y: spawn_object.y,
                    z: spawn_object.z,
                    pitch: spawn_object.pitch,
                    yaw: spawn_object.yaw,
                    data: spawn_object.data,
                    velocity_x: spawn_object.velocity_x,
                    velocity_y: spawn_object.velocity_y,
                    velocity_z: spawn_object.velocity_z,
                })
            }
            packet::Packet::SpawnObject_VarInt(spawn_object) => {
                mapped_packet::MappedPacket::SpawnObject(SpawnObject {
                    entity_id: spawn_object.entity_id.0,
                    uuid: Some(spawn_object.uuid),
                    ty: spawn_object.ty.0,
                    x: spawn_object.x,
                    y: spawn_object.y,
                    z: spawn_object.z,
                    pitch: spawn_object.pitch,
                    yaw: spawn_object.yaw,
                    data: spawn_object.data,
                    velocity_x: spawn_object.velocity_x,
                    velocity_y: spawn_object.velocity_y,
                    velocity_z: spawn_object.velocity_z,
                })
            }
            packet::Packet::SpawnObject_i32(spawn_object) => {
                mapped_packet::MappedPacket::SpawnObject(SpawnObject {
                    entity_id: spawn_object.entity_id.0,
                    uuid: Some(spawn_object.uuid),
                    ty: spawn_object.ty as i32,
                    x: From::from(spawn_object.x),
                    y: From::from(spawn_object.y),
                    z: From::from(spawn_object.z),
                    pitch: spawn_object.pitch,
                    yaw: spawn_object.yaw,
                    data: spawn_object.data,
                    velocity_x: spawn_object.velocity_x,
                    velocity_y: spawn_object.velocity_y,
                    velocity_z: spawn_object.velocity_z,
                })
            }
            packet::Packet::SpawnObject_i32_NoUUID(spawn_object) => {
                mapped_packet::MappedPacket::SpawnObject(SpawnObject {
                    entity_id: spawn_object.entity_id.0,
                    uuid: None,
                    ty: spawn_object.ty as i32,
                    x: From::from(spawn_object.x),
                    y: From::from(spawn_object.y),
                    z: From::from(spawn_object.z),
                    pitch: spawn_object.pitch,
                    yaw: spawn_object.yaw,
                    data: spawn_object.data,
                    velocity_x: spawn_object.velocity_x,
                    velocity_y: spawn_object.velocity_y,
                    velocity_z: spawn_object.velocity_z,
                })
            }
            packet::Packet::SetCurrentHotbarSlot(set_slot) => {
                mapped_packet::MappedPacket::SetCurrentHotbarSlot(SetCurrentHotbarSlot {
                    slot: set_slot.slot,
                })
            }
            packet::Packet::ServerMessage_Sender(server_msg) => {
                mapped_packet::MappedPacket::ServerMessage(ServerMessage {
                    message: server_msg.message,
                    position: Some(server_msg.position),
                    sender: Some(server_msg.sender),
                })
            }
            packet::Packet::ServerMessage_Position(server_msg) => {
                mapped_packet::MappedPacket::ServerMessage(ServerMessage {
                    message: server_msg.message,
                    position: Some(server_msg.position),
                    sender: None,
                })
            }
            packet::Packet::ServerMessage_NoPosition(server_msg) => {
                mapped_packet::MappedPacket::ServerMessage(ServerMessage {
                    message: server_msg.message,
                    position: None,
                    sender: None,
                })
            }
            packet::Packet::SpawnPlayer_f64(spawn_player) => {
                mapped_packet::MappedPacket::SpawnPlayer(SpawnPlayer {
                    entity_id: spawn_player.entity_id.0,
                    uuid: Some(spawn_player.uuid),
                    uuid_str: None,
                    name: None,
                    properties: None,
                    x: spawn_player.x,
                    y: spawn_player.y,
                    z: spawn_player.z,
                    yaw: spawn_player.yaw,
                    pitch: spawn_player.pitch,
                    current_item: None,
                    metadata: Some(spawn_player.metadata),
                })
            }
            packet::Packet::SpawnPlayer_f64_NoMeta(spawn_player) => {
                mapped_packet::MappedPacket::SpawnPlayer(SpawnPlayer {
                    entity_id: spawn_player.entity_id.0,
                    uuid: Some(spawn_player.uuid),
                    uuid_str: None,
                    name: None,
                    properties: None,
                    x: spawn_player.x,
                    y: spawn_player.y,
                    z: spawn_player.z,
                    yaw: spawn_player.yaw,
                    pitch: spawn_player.pitch,
                    current_item: None,
                    metadata: None,
                })
            }
            packet::Packet::SpawnPlayer_i32(spawn_player) => {
                mapped_packet::MappedPacket::SpawnPlayer(SpawnPlayer {
                    entity_id: spawn_player.entity_id.0,
                    uuid: Some(spawn_player.uuid),
                    uuid_str: None,
                    name: None,
                    properties: None,
                    x: From::from(spawn_player.x),
                    y: From::from(spawn_player.y),
                    z: From::from(spawn_player.z),
                    yaw: spawn_player.yaw,
                    pitch: spawn_player.pitch,
                    current_item: None,
                    metadata: Some(spawn_player.metadata),
                })
            }
            packet::Packet::SpawnPlayer_i32_HeldItem(spawn_player) => {
                mapped_packet::MappedPacket::SpawnPlayer(SpawnPlayer {
                    entity_id: spawn_player.entity_id.0,
                    uuid: Some(spawn_player.uuid),
                    uuid_str: None,
                    name: None,
                    properties: None,
                    x: From::from(spawn_player.x),
                    y: From::from(spawn_player.y),
                    z: From::from(spawn_player.z),
                    yaw: spawn_player.yaw,
                    pitch: spawn_player.pitch,
                    current_item: Some(spawn_player.current_item),
                    metadata: Some(spawn_player.metadata),
                })
            }
            packet::Packet::SpawnPlayer_i32_HeldItem_String(spawn_player) => {
                mapped_packet::MappedPacket::SpawnPlayer(SpawnPlayer {
                    entity_id: spawn_player.entity_id.0,
                    uuid: None,
                    uuid_str: Some(spawn_player.uuid),
                    name: Some(spawn_player.name),
                    properties: Some(spawn_player.properties.data),
                    x: From::from(spawn_player.x),
                    y: From::from(spawn_player.y),
                    z: From::from(spawn_player.z),
                    yaw: spawn_player.yaw,
                    pitch: spawn_player.pitch,
                    current_item: Some(spawn_player.current_item),
                    metadata: Some(spawn_player.metadata),
                })
            }
            packet::Packet::ScoreboardDisplay(display) => {
                mapped_packet::MappedPacket::ScoreboardDisplay(ScoreboardDisplay {
                    position: display.position,
                    name: display.name,
                })
            }
            packet::Packet::ScoreboardObjective(objective) => {
                mapped_packet::MappedPacket::ScoreboardObjective(ScoreboardObjective {
                    name: objective.name,
                    mode: Some(objective.mode),
                    value: objective.value,
                    ty_str: Some(objective.ty),
                    ty: None,
                })
            }
            packet::Packet::ScoreboardObjective_NoMode(objective) => {
                mapped_packet::MappedPacket::ScoreboardObjective(ScoreboardObjective {
                    name: objective.name,
                    mode: None,
                    value: objective.value,
                    ty_str: None,
                    ty: Some(objective.ty),
                })
            }
            packet::Packet::SelectAdvancementTab(advancements_tab) => {
                mapped_packet::MappedPacket::SelectAdvancementTab(SelectAdvancementTab {
                    has_id: advancements_tab.has_id,
                    tab_id: advancements_tab.tab_id,
                })
            }
            packet::Packet::SelectTrade(trade) => {
                mapped_packet::MappedPacket::SelectTrade(SelectTrade {
                    selected_slot: trade.selected_slot.0,
                })
            }
            packet::Packet::ServerDifficulty(difficulty) => {
                mapped_packet::MappedPacket::ServerDifficulty(ServerDifficulty {
                    difficulty: difficulty.difficulty,
                    locked: None,
                })
            }
            packet::Packet::ServerDifficulty_Locked(difficulty) => {
                mapped_packet::MappedPacket::ServerDifficulty(ServerDifficulty {
                    difficulty: difficulty.difficulty,
                    locked: Some(difficulty.locked),
                })
            }
            packet::Packet::SetBeaconEffect(beacon) => {
                mapped_packet::MappedPacket::SetBeaconEffect(SetBeaconEffect {
                    primary_effect: beacon.primary_effect.0,
                    secondary_effect: beacon.secondary_effect.0,
                })
            }
            packet::Packet::SetCompression(compression) => {
                mapped_packet::MappedPacket::SetCompression(SetCompression {
                    threshold: compression.threshold.0,
                })
            }
            packet::Packet::SetCooldown(cooldown) => {
                mapped_packet::MappedPacket::SetCooldown(SetCooldown {
                    item_id: cooldown.item_id.0,
                    ticks: cooldown.ticks.0,
                })
            }
            packet::Packet::SetDifficulty(difficulty) => {
                mapped_packet::MappedPacket::SetDifficulty(SetDifficulty {
                    new_difficulty: difficulty.new_difficulty,
                })
            }
            packet::Packet::SetDisplayedRecipe(displayed_recipe) => {
                mapped_packet::MappedPacket::SetDisplayedRecipe(SetDisplayedRecipe {
                    recipe_id: displayed_recipe.recipe_id,
                })
            }
            packet::Packet::SetExperience(set_exp) => {
                mapped_packet::MappedPacket::SetExperience(SetExperience {
                    experience_bar: set_exp.experience_bar,
                    level: set_exp.level.0,
                    total_experience: set_exp.total_experience.0,
                })
            }
            packet::Packet::SetExperience_i16(set_exp) => {
                mapped_packet::MappedPacket::SetExperience(SetExperience {
                    experience_bar: set_exp.experience_bar,
                    level: set_exp.level as i32,
                    total_experience: set_exp.total_experience as i32,
                })
            }
            packet::Packet::SetInitialCompression(init_comp) => {
                mapped_packet::MappedPacket::SetInitialCompression(SetInitialCompression {
                    threshold: init_comp.threshold.0,
                })
            }
            packet::Packet::SetPassengers(passengers) => {
                mapped_packet::MappedPacket::SetPassengers(SetPassengers {
                    entity_id: passengers.entity_id.0,
                    passengers: passengers.passengers.data.iter().map(|x| x.0).collect(),
                })
            }
            packet::Packet::SetRecipeBookState(recipe_book) => {
                mapped_packet::MappedPacket::SetRecipeBookState(SetRecipeBookState {
                    book_id: recipe_book.book_id.0,
                    book_open: recipe_book.book_open,
                    filter_active: recipe_book.filter_active,
                })
            }
            packet::Packet::SetSign(set_sign) => mapped_packet::MappedPacket::SetSign(SetSign {
                location: set_sign.location,
                line1: set_sign.line1,
                line2: set_sign.line2,
                line3: set_sign.line3,
                line4: set_sign.line4,
            }),
            packet::Packet::SetSign_i16y(set_sign) => {
                mapped_packet::MappedPacket::SetSign(SetSign {
                    location: Position::new(set_sign.x, set_sign.y as i32, set_sign.z),
                    line1: set_sign.line1,
                    line2: set_sign.line2,
                    line3: set_sign.line3,
                    line4: set_sign.line4,
                })
            }
            packet::Packet::SignEditorOpen(sign_editor) => {
                mapped_packet::MappedPacket::SignEditorOpen(SignEditorOpen {
                    location: sign_editor.location,
                })
            }
            packet::Packet::SignEditorOpen_i32(sign_editor) => {
                mapped_packet::MappedPacket::SignEditorOpen(SignEditorOpen {
                    location: Position::new(sign_editor.x, sign_editor.y, sign_editor.z),
                })
            }
            packet::Packet::SoundEffect(sound) => {
                mapped_packet::MappedPacket::SoundEffect(SoundEffect {
                    name: sound.name.0,
                    category: sound.category.0,
                    x: sound.x,
                    y: sound.y,
                    z: sound.z,
                    volume: sound.volume,
                    pitch: sound.pitch,
                })
            }
            packet::Packet::SoundEffect_u8(sound) => {
                mapped_packet::MappedPacket::SoundEffect(SoundEffect {
                    name: sound.name.0,
                    category: sound.category.0,
                    x: sound.x,
                    y: sound.y,
                    z: sound.z,
                    volume: sound.volume,
                    pitch: sound.pitch as f32, // TODO: Convert this somehow?
                })
            }
            packet::Packet::SpawnExperienceOrb(exp_orb) => {
                mapped_packet::MappedPacket::SpawnExperienceOrb(SpawnExperienceOrb {
                    entity_id: exp_orb.entity_id.0,
                    x: exp_orb.x,
                    y: exp_orb.y,
                    z: exp_orb.z,
                    count: exp_orb.count,
                })
            }
            packet::Packet::SpawnExperienceOrb_i32(exp_orb) => {
                mapped_packet::MappedPacket::SpawnExperienceOrb(SpawnExperienceOrb {
                    entity_id: exp_orb.entity_id.0,
                    x: From::from(exp_orb.x),
                    y: From::from(exp_orb.y),
                    z: From::from(exp_orb.z),
                    count: exp_orb.count,
                })
            }
            packet::Packet::SpawnGlobalEntity(global) => {
                mapped_packet::MappedPacket::SpawnGlobalEntity(SpawnGlobalEntity {
                    entity_id: global.entity_id.0,
                    ty: global.ty,
                    x: global.x,
                    y: global.y,
                    z: global.z,
                })
            }
            packet::Packet::SpawnGlobalEntity_i32(global) => {
                mapped_packet::MappedPacket::SpawnGlobalEntity(SpawnGlobalEntity {
                    entity_id: global.entity_id.0,
                    ty: global.ty,
                    x: From::from(global.x),
                    y: From::from(global.y),
                    z: From::from(global.z),
                })
            }
            packet::Packet::SpawnPainting_NoUUID(painting) => {
                mapped_packet::MappedPacket::SpawnPainting(SpawnPainting {
                    entity_id: painting.entity_id.0,
                    uuid: None,
                    motive: None,
                    title: Some(painting.title),
                    location: painting.location,
                    direction: painting.direction as i32,
                })
            }
            packet::Packet::SpawnPainting_NoUUID_i32(painting) => {
                mapped_packet::MappedPacket::SpawnPainting(SpawnPainting {
                    entity_id: painting.entity_id.0,
                    uuid: None,
                    motive: None,
                    title: Some(painting.title),
                    location: Position::new(painting.x, painting.y, painting.z),
                    direction: painting.direction,
                })
            }
            packet::Packet::SpawnPainting_String(painting) => {
                mapped_packet::MappedPacket::SpawnPainting(SpawnPainting {
                    entity_id: painting.entity_id.0,
                    uuid: Some(painting.uuid),
                    motive: None,
                    title: Some(painting.title),
                    location: painting.location,
                    direction: painting.direction as i32,
                })
            }
            packet::Packet::SpawnPainting_VarInt(painting) => {
                mapped_packet::MappedPacket::SpawnPainting(SpawnPainting {
                    entity_id: painting.entity_id.0,
                    uuid: Some(painting.uuid),
                    motive: Some(painting.motive.0),
                    title: None,
                    location: painting.location,
                    direction: painting.direction as i32,
                })
            }
            packet::Packet::SpawnPosition(position) => {
                mapped_packet::MappedPacket::SpawnPosition(SpawnPosition {
                    location: position.location,
                })
            }
            packet::Packet::SpawnPosition_i32(position) => {
                mapped_packet::MappedPacket::SpawnPosition(SpawnPosition {
                    location: Position::new(position.x, position.y, position.z),
                })
            }
            packet::Packet::SpectateTeleport(teleport) => {
                mapped_packet::MappedPacket::SpectateTeleport(SpectateTeleport {
                    target: teleport.target,
                })
            }
            packet::Packet::Statistics(statistics) => {
                mapped_packet::MappedPacket::Statistics(Statistics {
                    statistices: statistics.statistices.data,
                })
            }
            packet::Packet::StatusPing(ping) => {
                mapped_packet::MappedPacket::StatusPing(StatusPing { ping: ping.ping })
            }
            packet::Packet::StatusPong(pong) => {
                mapped_packet::MappedPacket::StatusPong(StatusPong { ping: pong.ping })
            }
            packet::Packet::StatusRequest(_request) => {
                mapped_packet::MappedPacket::StatusRequest(StatusRequest { empty: () })
            }
            packet::Packet::StatusResponse(response) => {
                mapped_packet::MappedPacket::StatusResponse(StatusResponse {
                    status: response.status,
                })
            }
            packet::Packet::SteerBoat(steer_boat) => {
                mapped_packet::MappedPacket::SteerBoat(SteerBoat {
                    left_paddle_turning: steer_boat.left_paddle_turning,
                    right_paddle_turning: steer_boat.right_paddle_turning,
                })
            }
            packet::Packet::SteerVehicle(steer_vehicle) => {
                mapped_packet::MappedPacket::SteerVehicle(SteerVehicle {
                    sideways: steer_vehicle.sideways,
                    forward: steer_vehicle.forward,
                    flags: Some(steer_vehicle.flags),
                    jump: None,
                    unmount: None,
                })
            }
            packet::Packet::SteerVehicle_jump_unmount(steer_vehicle) => {
                mapped_packet::MappedPacket::SteerVehicle(SteerVehicle {
                    sideways: steer_vehicle.sideways,
                    forward: steer_vehicle.forward,
                    flags: None,
                    jump: Some(steer_vehicle.jump),
                    unmount: Some(steer_vehicle.unmount),
                })
            }
            packet::Packet::StopSound(stop_sound) => {
                mapped_packet::MappedPacket::StopSound(StopSound {
                    flags: stop_sound.flags,
                    source: stop_sound.source.map(|x| x.0),
                    sound: stop_sound.sound,
                })
            }
            packet::Packet::TimeUpdate(time_update) => {
                mapped_packet::MappedPacket::TimeUpdate(TimeUpdate {
                    world_age: time_update.world_age,
                    time_of_day: time_update.time_of_day,
                })
            }
            packet::Packet::TeleportConfirm(teleport_confirm) => {
                mapped_packet::MappedPacket::TeleportConfirm(TeleportConfirm {
                    teleport_id: teleport_confirm.teleport_id.0,
                })
            }
            packet::Packet::TeleportPlayer_OnGround(tp_player) => {
                mapped_packet::MappedPacket::TeleportPlayer(TeleportPlayer {
                    x: tp_player.x,
                    y: tp_player.eyes_y - 1.62,
                    z: tp_player.z,
                    yaw: tp_player.yaw,
                    pitch: tp_player.pitch,
                    flags: None,
                    teleport_id: None,
                    on_ground: Some(tp_player.on_ground),
                })
            }
            packet::Packet::TeleportPlayer_NoConfirm(tp_player) => {
                mapped_packet::MappedPacket::TeleportPlayer(TeleportPlayer {
                    x: tp_player.x,
                    y: tp_player.y,
                    z: tp_player.z,
                    yaw: tp_player.yaw,
                    pitch: tp_player.pitch,
                    flags: Some(tp_player.flags),
                    teleport_id: None,
                    on_ground: None,
                })
            }
            packet::Packet::TeleportPlayer_WithConfirm(tp_player) => {
                mapped_packet::MappedPacket::TeleportPlayer(TeleportPlayer {
                    x: tp_player.x,
                    y: tp_player.y,
                    z: tp_player.z,
                    yaw: tp_player.yaw,
                    pitch: tp_player.pitch,
                    flags: Some(tp_player.flags),
                    teleport_id: Some(tp_player.teleport_id.0),
                    on_ground: None,
                })
            }
            packet::Packet::TabComplete(tab_complete) => {
                mapped_packet::MappedPacket::TabComplete(TabComplete {
                    text: tab_complete.text,
                    assume_command: Some(tab_complete.assume_command),
                    has_target: Some(tab_complete.has_target),
                    target: tab_complete.target,
                })
            }
            packet::Packet::TabComplete_NoAssume(tab_complete) => {
                mapped_packet::MappedPacket::TabComplete(TabComplete {
                    text: tab_complete.text,
                    assume_command: None,
                    has_target: Some(tab_complete.has_target),
                    target: tab_complete.target,
                })
            }
            packet::Packet::TabComplete_NoAssume_NoTarget(tab_complete) => {
                mapped_packet::MappedPacket::TabComplete(TabComplete {
                    text: tab_complete.text,
                    assume_command: None,
                    has_target: None,
                    target: None,
                })
            }
            packet::Packet::TabCompleteReply(reply) => {
                mapped_packet::MappedPacket::TabCompleteReply(TabCompleteReply {
                    matches: reply.matches.data,
                })
            }
            packet::Packet::Tags(tags) => mapped_packet::MappedPacket::Tags(Tags {
                block_tags: tags.block_tags.data,
                item_tags: tags.item_tags.data,
                fluid_tags: tags.fluid_tags.data,
                entity_tags: None,
            }),
            packet::Packet::TagsWithEntities(tags) => mapped_packet::MappedPacket::Tags(Tags {
                block_tags: tags.block_tags.data,
                item_tags: tags.item_tags.data,
                fluid_tags: tags.fluid_tags.data,
                entity_tags: Some(tags.entity_tags.data),
            }),
            packet::Packet::Teams_u8(teams) => mapped_packet::MappedPacket::Teams(Teams {
                name: teams.name,
                mode: teams.mode,
                display_name: teams.display_name,
                flags: teams.flags,
                name_tag_visibility: teams.name_tag_visibility,
                collision_rule: teams.collision_rule,
                formatting: None,
                prefix: teams.prefix,
                suffix: teams.suffix,
                players: teams.players.map(|x| x.data),
                color: teams.color,
                data: Some(teams.data),
            }),
            packet::Packet::Teams_NoVisColor(teams) => mapped_packet::MappedPacket::Teams(Teams {
                name: teams.name,
                mode: teams.mode,
                display_name: teams.display_name,
                flags: teams.flags,
                name_tag_visibility: None,
                collision_rule: None,
                formatting: None,
                prefix: teams.prefix,
                suffix: teams.suffix,
                players: teams.players.map(|x| x.data),
                color: None,
                data: None,
            }),
            packet::Packet::Teams_VarInt(teams) => mapped_packet::MappedPacket::Teams(Teams {
                name: teams.name,
                mode: teams.mode,
                display_name: teams.display_name,
                flags: teams.flags,
                name_tag_visibility: teams.name_tag_visibility,
                collision_rule: teams.collision_rule,
                formatting: teams.formatting.map(|x| x.0),
                prefix: teams.prefix,
                suffix: teams.suffix,
                players: teams.players.map(|x| x.data),
                color: None,
                data: None,
            }),
            packet::Packet::Title(title) => mapped_packet::MappedPacket::Title(Title {
                action: title.action.0,
                title: title.title,
                sub_title: title.sub_title,
                action_bar_text: title.action_bar_text,
                fade_in: title.fade_in,
                fade_stay: title.fade_stay,
                fade_out: title.fade_out,
                fade_in_comp: None,
                fade_stay_comp: None,
                fade_out_comp: None,
            }),
            packet::Packet::Title_notext(title) => mapped_packet::MappedPacket::Title(Title {
                action: title.action.0,
                title: title.title,
                sub_title: title.sub_title,
                action_bar_text: None,
                fade_in: title.fade_in,
                fade_stay: title.fade_stay,
                fade_out: title.fade_out,
                fade_in_comp: None,
                fade_stay_comp: None,
                fade_out_comp: None,
            }),
            packet::Packet::Title_notext_component(title) => {
                mapped_packet::MappedPacket::Title(Title {
                    action: title.action.0,
                    title: title.title,
                    sub_title: title.sub_title,
                    action_bar_text: None,
                    fade_in: None,
                    fade_stay: None,
                    fade_out: None,
                    fade_in_comp: title.fade_in,
                    fade_stay_comp: title.fade_stay,
                    fade_out_comp: title.fade_out,
                })
            }
            packet::Packet::TradeList_WithoutRestock(trade_list) => {
                mapped_packet::MappedPacket::TradeList(TradeList {
                    id: trade_list.id.0,
                    trades: trade_list.trades.data,
                    villager_level: trade_list.villager_level.0,
                    experience: trade_list.experience.0,
                    is_regular_villager: trade_list.is_regular_villager,
                    can_restock: None,
                })
            }
            packet::Packet::TradeList_WithRestock(trade_list) => {
                mapped_packet::MappedPacket::TradeList(TradeList {
                    id: trade_list.id.0,
                    trades: trade_list.trades.data,
                    villager_level: trade_list.villager_level.0,
                    experience: trade_list.experience.0,
                    is_regular_villager: trade_list.is_regular_villager,
                    can_restock: Some(trade_list.can_restock),
                })
            }
            packet::Packet::UpdateHealth(health) => {
                mapped_packet::MappedPacket::UpdateHealth(UpdateHealth {
                    health: health.health,
                    food: health.food.0,
                    food_saturation: health.food_saturation,
                })
            }
            packet::Packet::UpdateHealth_u16(health) => {
                mapped_packet::MappedPacket::UpdateHealth(UpdateHealth {
                    health: health.health,
                    food: health.food as i32,
                    food_saturation: health.food_saturation,
                })
            }
            packet::Packet::UpdateLight_WithTrust(light) => {
                mapped_packet::MappedPacket::UpdateLight(UpdateLight {
                    chunk_x: light.chunk_x.0,
                    chunk_z: light.chunk_z.0,
                    trust_edges: Some(light.trust_edges),
                    sky_light_mask: light.sky_light_mask.0,
                    block_light_mask: light.block_light_mask.0,
                    empty_sky_light_mask: light.empty_sky_light_mask.0,
                    light_arrays: light.light_arrays,
                })
            }
            packet::Packet::UpdateLight_NoTrust(light) => {
                mapped_packet::MappedPacket::UpdateLight(UpdateLight {
                    chunk_x: light.chunk_x.0,
                    chunk_z: light.chunk_z.0,
                    trust_edges: None,
                    sky_light_mask: light.sky_light_mask.0,
                    block_light_mask: light.block_light_mask.0,
                    empty_sky_light_mask: light.empty_sky_light_mask.0,
                    light_arrays: light.light_arrays,
                })
            }
            packet::Packet::UpdateViewPosition(view_position) => {
                mapped_packet::MappedPacket::UpdateViewPosition(UpdateViewPosition {
                    chunk_x: view_position.chunk_x.0,
                    chunk_z: view_position.chunk_z.0,
                })
            }
            packet::Packet::UpdateBlockEntity(block_entity) => {
                mapped_packet::MappedPacket::UpdateBlockEntity(UpdateBlockEntity {
                    location: block_entity.location,
                    action: block_entity.action,
                    nbt: block_entity.nbt,
                    data_length: None,
                    gzipped_nbt: None,
                })
            }
            packet::Packet::UpdateBlockEntity_Data(block_entity) => {
                mapped_packet::MappedPacket::UpdateBlockEntity(UpdateBlockEntity {
                    location: Position::new(block_entity.x, block_entity.y as i32, block_entity.z),
                    action: block_entity.action,
                    nbt: None,
                    data_length: Some(block_entity.data_length),
                    gzipped_nbt: Some(block_entity.gzipped_nbt),
                })
            }
            packet::Packet::UpdateSign(sign) => {
                mapped_packet::MappedPacket::UpdateSign(UpdateSign {
                    location: sign.location,
                    line1: sign.line1,
                    line2: sign.line2,
                    line3: sign.line3,
                    line4: sign.line4,
                })
            }
            packet::Packet::UpdateSign_u16(sign) => {
                mapped_packet::MappedPacket::UpdateSign(UpdateSign {
                    location: Position::new(sign.x, sign.y as i32, sign.z),
                    line1: sign.line1,
                    line2: sign.line2,
                    line3: sign.line3,
                    line4: sign.line4,
                })
            }
            packet::Packet::UnlockRecipes_NoSmelting(recipes) => {
                mapped_packet::MappedPacket::UnlockRecipes(UnlockRecipes {
                    action: recipes.action.0,
                    crafting_book_open: recipes.crafting_book_open,
                    filtering_craftable: recipes.filtering_craftable,
                    smelting_book_open: None,
                    filtering_smeltable: None,
                    blast_furnace_open: None,
                    filtering_blast_furnace: None,
                    smoker_open: None,
                    filtering_smoker: None,
                    recipe_ids: Some(recipes.recipe_ids.data.iter().map(|x| x.0).collect()),
                    recipe_ids2: Some(recipes.recipe_ids2.data.iter().map(|x| x.0).collect()),
                    recipe_ids_str: None,
                    recipe_ids_str2: None,
                })
            }
            packet::Packet::UnlockRecipes_WithSmelting(recipes) => {
                mapped_packet::MappedPacket::UnlockRecipes(UnlockRecipes {
                    action: recipes.action.0,
                    crafting_book_open: recipes.crafting_book_open,
                    filtering_craftable: recipes.filtering_craftable,
                    smelting_book_open: Some(recipes.smelting_book_open),
                    filtering_smeltable: Some(recipes.filtering_smeltable),
                    blast_furnace_open: None,
                    filtering_blast_furnace: None,
                    smoker_open: None,
                    filtering_smoker: None,
                    recipe_ids: None,
                    recipe_ids2: None,
                    recipe_ids_str: Some(recipes.recipe_ids.data),
                    recipe_ids_str2: Some(recipes.recipe_ids2.data),
                })
            }
            packet::Packet::UnlockRecipes_WithBlastSmoker(recipes) => {
                mapped_packet::MappedPacket::UnlockRecipes(UnlockRecipes {
                    action: recipes.action.0,
                    crafting_book_open: recipes.crafting_book_open,
                    filtering_craftable: recipes.filtering_craftable,
                    smelting_book_open: Some(recipes.smelting_book_open),
                    filtering_smeltable: Some(recipes.filtering_smeltable),
                    blast_furnace_open: Some(recipes.blast_furnace_open),
                    filtering_blast_furnace: Some(recipes.filtering_blast_furnace),
                    smoker_open: Some(recipes.smoker_open),
                    filtering_smoker: Some(recipes.filtering_smoker),
                    recipe_ids: None,
                    recipe_ids2: None,
                    recipe_ids_str: Some(recipes.recipe_ids.data),
                    recipe_ids_str2: Some(recipes.recipe_ids2.data),
                })
            }
            packet::Packet::UpdateCommandBlock(command) => {
                mapped_packet::MappedPacket::UpdateCommandBlock(UpdateCommandBlock {
                    location: command.location,
                    command: command.command,
                    mode: command.mode.0,
                    flags: command.flags,
                })
            }
            packet::Packet::UpdateCommandBlockMinecart(command_minecart) => {
                mapped_packet::MappedPacket::UpdateCommandBlockMinecart(
                    UpdateCommandBlockMinecart {
                        entity_id: command_minecart.entity_id.0,
                        command: command_minecart.command,
                        track_output: command_minecart.track_output,
                    },
                )
            }
            packet::Packet::UpdateJigsawBlock_Joint(jigsaw) => {
                mapped_packet::MappedPacket::UpdateJigsawBlock_Joint(UpdateJigsawBlock_Joint {
                    location: jigsaw.location,
                    name: jigsaw.name,
                    target: jigsaw.target,
                    pool: jigsaw.pool,
                    final_state: jigsaw.final_state,
                    joint_type: jigsaw.joint_type,
                })
            }
            packet::Packet::UpdateJigsawBlock_Type(jigsaw) => {
                mapped_packet::MappedPacket::UpdateJigsawBlock_Type(UpdateJigsawBlock_Type {
                    location: jigsaw.location,
                    attachment_type: jigsaw.attachment_type,
                    target_pool: jigsaw.target_pool,
                    final_state: jigsaw.final_state,
                })
            }
            packet::Packet::UpdateScore(score) => {
                mapped_packet::MappedPacket::UpdateScore(UpdateScore {
                    name: score.name,
                    action: score.action,
                    object_name: score.object_name,
                    value: score.value.map(|x| x.0),
                })
            }
            packet::Packet::UpdateScore_i32(score) => {
                mapped_packet::MappedPacket::UpdateScore(UpdateScore {
                    name: score.name,
                    action: score.action,
                    object_name: score.object_name,
                    value: score.value,
                })
            }
            packet::Packet::UpdateStructureBlock(structure_block) => {
                mapped_packet::MappedPacket::UpdateStructureBlock(UpdateStructureBlock {
                    location: structure_block.location,
                    action: structure_block.action.0,
                    mode: structure_block.mode.0,
                    name: structure_block.name,
                    offset_x: structure_block.offset_x,
                    offset_y: structure_block.offset_y,
                    offset_z: structure_block.offset_z,
                    size_x: structure_block.size_x,
                    size_y: structure_block.size_y,
                    size_z: structure_block.size_z,
                    mirror: structure_block.mirror.0,
                    rotation: structure_block.rotation.0,
                    metadata: structure_block.metadata,
                    integrity: structure_block.integrity,
                    seed: structure_block.seed.0,
                    flags: structure_block.flags,
                })
            }
            packet::Packet::UpdateViewDistance(view_distance) => {
                mapped_packet::MappedPacket::UpdateViewDistance(UpdateViewDistance {
                    view_distance: view_distance.view_distance.0,
                })
            }
            packet::Packet::UseEntity_Hand(use_entity) => {
                mapped_packet::MappedPacket::UseEntity(UseEntity {
                    target_id: use_entity.target_id.0,
                    ty: use_entity.ty.0,
                    target_x: Some(use_entity.target_x),
                    target_y: Some(use_entity.target_y),
                    target_z: Some(use_entity.target_z),
                    hand: Some(use_entity.hand.0),
                    sneaking: None,
                })
            }
            packet::Packet::UseEntity_Handsfree(use_entity) => {
                mapped_packet::MappedPacket::UseEntity(UseEntity {
                    target_id: use_entity.target_id.0,
                    ty: use_entity.ty.0,
                    target_x: Some(use_entity.target_x),
                    target_y: Some(use_entity.target_y),
                    target_z: Some(use_entity.target_z),
                    hand: None,
                    sneaking: None,
                })
            }
            packet::Packet::UseEntity_Handsfree_i32(use_entity) => {
                mapped_packet::MappedPacket::UseEntity(UseEntity {
                    target_id: use_entity.target_id,
                    ty: use_entity.ty as i32,
                    target_x: None,
                    target_y: None,
                    target_z: None,
                    hand: None,
                    sneaking: None,
                })
            }
            packet::Packet::UseEntity_Sneakflag(use_entity) => {
                mapped_packet::MappedPacket::UseEntity(UseEntity {
                    target_id: use_entity.target_id.0,
                    ty: use_entity.ty.0,
                    target_x: Some(use_entity.target_x),
                    target_y: Some(use_entity.target_y),
                    target_z: Some(use_entity.target_z),
                    hand: Some(use_entity.hand.0),
                    sneaking: Some(use_entity.sneaking),
                })
            }
            packet::Packet::UseItem(use_item) => mapped_packet::MappedPacket::UseItem(UseItem {
                hand: use_item.hand.0,
            }),
            packet::Packet::VehicleMove(vehicle_move) => {
                mapped_packet::MappedPacket::VehicleMove(VehicleMove {
                    x: vehicle_move.x,
                    y: vehicle_move.y,
                    z: vehicle_move.z,
                    yaw: vehicle_move.yaw,
                    pitch: vehicle_move.pitch,
                })
            }
            packet::Packet::VehicleTeleport(teleport) => {
                mapped_packet::MappedPacket::VehicleTeleport(VehicleTeleport {
                    x: teleport.x,
                    y: teleport.y,
                    z: teleport.z,
                    yaw: teleport.yaw,
                    pitch: teleport.pitch,
                })
            }
            packet::Packet::WindowItems(items) => {
                mapped_packet::MappedPacket::WindowItems(WindowItems {
                    id: items.id,
                    items: items.items.data,
                })
            }
            packet::Packet::WindowClose(close) => {
                mapped_packet::MappedPacket::WindowClose(WindowClose { id: close.id })
            }
            packet::Packet::WindowOpen(open) => {
                mapped_packet::MappedPacket::WindowOpen(WindowOpen {
                    id: open.id as i32,
                    ty: None,
                    ty_name: Some(open.ty),
                    title: open.title,
                    slot_count: Some(open.slot_count),
                    use_provided_window_title: None,
                    entity_id: Some(open.entity_id),
                })
            }
            packet::Packet::WindowOpen_u8(open) => {
                mapped_packet::MappedPacket::WindowOpen(WindowOpen {
                    id: open.id as i32,
                    ty: Some(open.ty as i32),
                    ty_name: None,
                    title: open.title,
                    slot_count: Some(open.slot_count),
                    use_provided_window_title: Some(open.use_provided_window_title),
                    entity_id: Some(open.entity_id),
                })
            }
            packet::Packet::WindowOpen_VarInt(open) => {
                mapped_packet::MappedPacket::WindowOpen(WindowOpen {
                    id: open.id.0,
                    ty: Some(open.ty.0),
                    ty_name: None,
                    title: open.title,
                    slot_count: None,
                    use_provided_window_title: None,
                    entity_id: None,
                })
            }
            packet::Packet::WindowOpenHorse(open) => {
                mapped_packet::MappedPacket::WindowOpenHorse(WindowOpenHorse {
                    window_id: open.window_id,
                    number_of_slots: open.number_of_slots.0,
                    entity_id: open.entity_id,
                })
            }
            packet::Packet::WindowProperty(property) => {
                mapped_packet::MappedPacket::WindowProperty(WindowProperty {
                    id: property.id,
                    property: property.property,
                    value: property.value,
                })
            }
            packet::Packet::WindowSetSlot(set_slot) => {
                mapped_packet::MappedPacket::WindowSetSlot(WindowSetSlot {
                    id: set_slot.id,
                    slot: set_slot.slot,
                    item: set_slot.item,
                })
            }
            packet::Packet::WorldBorder(border) => {
                mapped_packet::MappedPacket::WorldBorder(WorldBorder {
                    action: border.action.0,
                    old_radius: border.old_radius,
                    new_radius: border.new_radius,
                    speed: border.speed.map(|x| x.0),
                    x: border.x,
                    z: border.z,
                    portal_boundary: border.portal_boundary.map(|x| x.0),
                    warning_time: border.warning_time.map(|x| x.0),
                    warning_blocks: border.warning_blocks.map(|x| x.0),
                })
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct BlockChangeRecord {
    pub xz: u8,
    pub y: u8,
    pub block_id: i32,
}
