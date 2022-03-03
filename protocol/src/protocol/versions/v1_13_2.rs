// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

protocol_packet_ids!(
    handshake Handshaking {
        serverbound Serverbound {
            0x00 => Handshake
        }
        clientbound Clientbound {
        }
    }
    play Play {
        serverbound Serverbound {
            0x00 => TeleportConfirm
            0x01 => QueryBlockNBT
            0x02 => ChatMessage
            0x03 => ClientStatus
            0x04 => ClientSettings
            0x05 => TabComplete
            0x06 => ConfirmTransactionServerbound
            0x07 => EnchantItem
            0x08 => ClickWindow
            0x09 => CloseWindow
            0x0a => PluginMessageServerbound
            0x0b => EditBook
            0x0c => QueryEntityNBT
            0x0d => UseEntity_Hand
            0x0e => KeepAliveServerbound_i64
            0x0f => Player
            0x10 => PlayerPosition
            0x11 => PlayerPositionLook
            0x12 => PlayerLook
            0x13 => VehicleMove
            0x14 => SteerBoat
            0x15 => PickItem
            0x16 => CraftRecipeRequest
            0x17 => ClientAbilities_f32
            0x18 => PlayerDigging
            0x19 => PlayerAction
            0x1a => SteerVehicle
            0x1b => CraftingBookData
            0x1c => NameItem
            0x1d => ResourcePackStatus
            0x1e => AdvancementTab
            0x1f => SelectTrade
            0x20 => SetBeaconEffect
            0x21 => HeldItemChange
            0x22 => UpdateCommandBlock
            0x23 => UpdateCommandBlockMinecart
            0x24 => CreativeInventoryAction
            0x25 => UpdateStructureBlock
            0x26 => SetSign
            0x27 => ArmSwing
            0x28 => SpectateTeleport
            0x29 => PlayerBlockPlacement_f32
            0x2a => UseItem
        }
        clientbound Clientbound {
            0x00 => SpawnObject
            0x01 => SpawnExperienceOrb
            0x02 => SpawnGlobalEntity
            0x03 => SpawnMob_WithMeta
            0x04 => SpawnPainting_VarInt
            0x05 => SpawnPlayer_f64
            0x06 => Animation
            0x07 => Statistics
            0x08 => BlockBreakAnimation
            0x09 => UpdateBlockEntity
            0x0a => BlockAction
            0x0b => BlockChange_VarInt
            0x0c => BossBar
            0x0d => ServerDifficulty
            0x0e => ServerMessage_Position
            0x0f => MultiBlockChange_VarInt
            0x10 => TabCompleteReply
            0x11 => DeclareCommands
            0x12 => ConfirmTransaction
            0x13 => WindowClose
            0x14 => WindowOpen
            0x15 => WindowItems
            0x16 => WindowProperty
            0x17 => WindowSetSlot
            0x18 => SetCooldown
            0x19 => PluginMessageClientbound
            0x1a => NamedSoundEffect
            0x1b => Disconnect
            0x1c => EntityAction
            0x1d => NBTQueryResponse
            0x1e => Explosion
            0x1f => ChunkUnload
            0x20 => ChangeGameState
            0x21 => KeepAliveClientbound_i64
            0x22 => ChunkData
            0x23 => Effect
            0x24 => Particle_Data13
            0x25 => JoinGame_i32
            0x26 => Maps_NoLocked
            0x27 => Entity
            0x28 => EntityMove_i16
            0x29 => EntityLookAndMove_i16
            0x2a => EntityLook_VarInt
            0x2b => VehicleTeleport
            0x2c => SignEditorOpen
            0x2d => CraftRecipeResponse
            0x2e => PlayerAbilities
            0x2f => CombatEvent
            0x30 => PlayerInfo
            0x31 => FacePlayer
            0x32 => TeleportPlayer_WithConfirm
            0x33 => EntityUsedBed
            0x34 => UnlockRecipes_WithSmelting
            0x35 => EntityDestroy
            0x36 => EntityRemoveEffect
            0x37 => ResourcePackSend
            0x38 => Respawn_Gamemode
            0x39 => EntityHeadLook
            0x3a => SelectAdvancementTab
            0x3b => WorldBorder
            0x3c => Camera
            0x3d => SetCurrentHotbarSlot
            0x3e => ScoreboardDisplay
            0x3f => EntityMetadata
            0x40 => EntityAttach
            0x41 => EntityVelocity
            0x42 => EntityEquipment_VarInt
            0x43 => SetExperience
            0x44 => UpdateHealth
            0x45 => ScoreboardObjective
            0x46 => SetPassengers
            0x47 => Teams_VarInt
            0x48 => UpdateScore
            0x49 => SpawnPosition
            0x4a => TimeUpdate
            0x4c => StopSound
            0x4d => SoundEffect
            0x4e => PlayerListHeaderFooter
            0x4f => CollectItem
            0x50 => EntityTeleport_f64
            0x51 => Advancements
            0x52 => EntityProperties
            0x53 => EntityEffect
            0x54 => DeclareRecipes
            0x55 => Tags
        }
    }
    login Login {
        serverbound Serverbound {
            0x00 => LoginStart
            0x01 => EncryptionResponse
            0x02 => LoginPluginResponse
        }
        clientbound Clientbound {
            0x00 => LoginDisconnect
            0x01 => EncryptionRequest
            0x02 => LoginSuccess_String
            0x03 => SetInitialCompression
            0x04 => LoginPluginRequest
        }
    }
    status Status {
        serverbound Serverbound {
            0x00 => StatusRequest
            0x01 => StatusPing
        }
        clientbound Clientbound {
            0x00 => StatusResponse
            0x01 => StatusPong
        }
    }
);
