#![recursion_limit = "600"]
#![allow(clippy::identity_op)]
#![allow(clippy::collapsible_if)]

extern crate leafish_shared as shared;

use crate::shared::{Axis, Direction, Position, Version};
use cgmath::Point3;
use collision::Aabb3;
use std::collections::HashMap;

pub mod material;
pub use self::material::Material;
#[rustfmt::skip] mod blocks;
#[rustfmt::skip] mod versions;

pub use self::blocks::Block::*;
pub use self::blocks::*;

pub trait WorldAccess {
    fn get_block(&self, pos: Position) -> Block;
}

enum IDMapKind {
    Flat(&'static [Block]),
    Hierarchical,
}

pub struct VanillaIDMap {
    mapping: IDMapKind,
    modded: HashMap<String, [Option<Block>; 16]>,
}

impl VanillaIDMap {
    pub fn new(protocol_version: i32) -> VanillaIDMap {
        let version = Version::from_id(protocol_version as u32);
        let mapping = if version >= Version::V1_13 {
            IDMapKind::Flat(versions::get_block_mapping(version))
        } else {
            IDMapKind::Hierarchical
        };

        Self {
            mapping,
            modded: HashMap::new(),
        }
    }

    pub fn by_vanilla_id(
        &self,
        id: usize,
        modded_block_ids: &HashMap<usize, String>, // TODO: remove and add to constructor, but have to mutate in Server
    ) -> Block {
        match &self.mapping {
            IDMapKind::Flat(blocks) => {
                blocks.get(id).copied().unwrap_or(Block::Missing {})
                // TODO: support modded 1.13.2+ blocks after https://github.com/iceiix/stevenarella/pull/145
            }
            IDMapKind::Hierarchical => {
                if let Some(block) = versions::legacy::resolve(id) {
                    block
                } else {
                    let data = id & 0xf;
                    if let Some(name) = modded_block_ids.get(&(id >> 4)) {
                        if let Some(blocks_by_data) = self.modded.get(name) {
                            blocks_by_data[data].unwrap_or(Block::Missing {})
                        } else {
                            //info!("Modded block not supported yet: {}:{} -> {}", id >> 4, data, name);
                            Block::Missing {}
                        }
                    } else {
                        Block::Missing {}
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TintType {
    Default,
    Color { r: u8, g: u8, b: u8 },
    Grass,
    Foliage,
    Water,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    // Spot check a few blocks across different versions, including the correctly recognized last supported block
    // TODO: comprehensive testing against https://github.com/PrismarineJS/minecraft-data/tree/master/data/pc

    #[test]
    fn hier_1_12_2() {
        let id_map = VanillaIDMap::new(340);
        assert_eq!(
            id_map.by_vanilla_id(255 << 4, &Arc::new(HashMap::new())),
            StructureBlock {
                mode: StructureBlockMode::Save
            }
        );
        assert_eq!(
            id_map.by_vanilla_id((255 << 4) | 3, &Arc::new(HashMap::new())),
            StructureBlock {
                mode: StructureBlockMode::Data
            }
        );
    }

    #[test]
    fn flat_1_13_2() {
        let id_map = VanillaIDMap::new(404);
        assert_eq!(
            id_map.by_vanilla_id(8595, &Arc::new(HashMap::new())),
            StructureBlock {
                mode: StructureBlockMode::Save
            }
        );
        assert_eq!(
            id_map.by_vanilla_id(8598, &Arc::new(HashMap::new())),
            StructureBlock {
                mode: StructureBlockMode::Data
            }
        );
    }

    #[test]
    fn flat_1_14_4() {
        let id_map = VanillaIDMap::new(477);
        assert_eq!(
            id_map.by_vanilla_id(9113, &Arc::new(HashMap::new())),
            Conduit { waterlogged: true }
        );
        assert_eq!(
            id_map.by_vanilla_id(9114, &Arc::new(HashMap::new())),
            Conduit { waterlogged: false }
        );
    }

    #[test]
    fn flat_1_15_1() {
        let id_map = VanillaIDMap::new(575);
        assert_eq!(
            id_map.by_vanilla_id(9113, &Arc::new(HashMap::new())),
            Conduit { waterlogged: true }
        );
        assert_eq!(
            id_map.by_vanilla_id(9114, &Arc::new(HashMap::new())),
            Conduit { waterlogged: false }
        );
    }

    #[test]
    fn flat_1_16() {
        let id_map = VanillaIDMap::new(735);
        assert_eq!(
            id_map.by_vanilla_id(1048, &Arc::new(HashMap::new())),
            NoteBlock {
                instrument: NoteBlockInstrument::Pling,
                note: 24,
                powered: false
            }
        );
    }

    #[test]
    fn flat_1_16_2() {
        let id_map = VanillaIDMap::new(751);
        assert_eq!(
            id_map.by_vanilla_id(1048, &Arc::new(HashMap::new())),
            NoteBlock {
                instrument: NoteBlockInstrument::Pling,
                note: 24,
                powered: false
            }
        );
    }

    #[test]
    fn verify_blocks() {
        let dirt = Block::Dirt {};
        let stone = Block::Stone {};
        let vine = Block::Vine {
            up: false,
            south: false,
            west: false,
            north: true,
            east: false,
        };
        let pumpkin_lit = Block::JackOLantern {
            facing: Direction::North,
        };
        let cocoa = Block::Cocoa {
            age: 1,
            facing: Direction::North,
        };
        let leaves = Block::OakLeaves {
            distance: 1,
            persistent: true,
            waterlogged: false,
        };
        let wool = Block::WhiteWool {};
        let tall_seagrass = Block::TallSeagrass {
            half: TallSeagrassHalf::Upper,
        };
        let data = [
            (dirt, None, Some(0.75)),
            (dirt, Some(Tool::Shovel(ToolMaterial::Wooden)), Some(0.4)),
            (dirt, Some(Tool::Pickaxe(ToolMaterial::Wooden)), Some(0.75)),
            (stone, None, Some(7.5)),
            (stone, Some(Tool::Shovel(ToolMaterial::Wooden)), Some(7.5)),
            (stone, Some(Tool::Pickaxe(ToolMaterial::Wooden)), Some(1.15)),
            (Block::Obsidian {}, None, Some(250.0)),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Wooden)),
                Some(125.0),
            ),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Stone)),
                Some(62.5),
            ),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Iron)),
                Some(41.7),
            ),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Diamond)),
                Some(9.4),
            ),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Netherite)),
                Some(8.35),
            ),
            (
                Block::Obsidian {},
                Some(Tool::Pickaxe(ToolMaterial::Golden)),
                Some(20.85),
            ),
            (Block::Bedrock {}, None, None),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Wooden)),
                None,
            ),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Stone)),
                None,
            ),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Iron)),
                None,
            ),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Diamond)),
                None,
            ),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Netherite)),
                None,
            ),
            (
                Block::Bedrock {},
                Some(Tool::Pickaxe(ToolMaterial::Golden)),
                None,
            ),
            (Block::Cobweb {}, None, Some(20.0)),
            (
                Block::Cobweb {},
                Some(Tool::Pickaxe(ToolMaterial::Wooden)),
                Some(20.0),
            ),
            (vine, None, Some(0.3)),
            (vine, Some(Tool::Pickaxe(ToolMaterial::Wooden)), Some(0.3)),
            (vine, Some(Tool::Axe(ToolMaterial::Wooden)), Some(0.15)),
            (vine, Some(Tool::Axe(ToolMaterial::Stone)), Some(0.1)),
            (vine, Some(Tool::Axe(ToolMaterial::Iron)), Some(0.05)),
            (vine, Some(Tool::Axe(ToolMaterial::Diamond)), Some(0.05)),
            (wool, None, Some(1.2)),
            (leaves, None, Some(0.3)),
            (leaves, Some(Tool::Hoe(ToolMaterial::Wooden)), Some(0.15)),
            (leaves, Some(Tool::Hoe(ToolMaterial::Stone)), Some(0.1)),
            (leaves, Some(Tool::Hoe(ToolMaterial::Iron)), Some(0.05)),
            (leaves, Some(Tool::Hoe(ToolMaterial::Diamond)), Some(0.05)),
            (Block::DeadBush {}, None, Some(0.05)),
            (Block::DeadBush {}, Some(Tool::Shears), Some(0.05)),
            (Block::Seagrass {}, None, Some(0.05)),
            (Block::Seagrass {}, Some(Tool::Shears), Some(0.05)),
            (tall_seagrass, None, Some(0.05)),
            (tall_seagrass, Some(Tool::Shears), Some(0.05)),
            (cocoa, None, Some(0.3)),
            (cocoa, Some(Tool::Axe(ToolMaterial::Wooden)), Some(0.15)),
            (cocoa, Some(Tool::Axe(ToolMaterial::Stone)), Some(0.1)),
            (cocoa, Some(Tool::Axe(ToolMaterial::Iron)), Some(0.05)),
            (cocoa, Some(Tool::Axe(ToolMaterial::Diamond)), Some(0.05)),
            (Block::Melon {}, None, Some(1.5)),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Wooden)),
                Some(0.75),
            ),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Stone)),
                Some(0.4),
            ),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Iron)),
                Some(0.25),
            ),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Diamond)),
                Some(0.2),
            ),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Netherite)),
                Some(0.2),
            ),
            (
                Block::Melon {},
                Some(Tool::Axe(ToolMaterial::Golden)),
                Some(0.15),
            ),
            (Block::Pumpkin {}, None, Some(1.5)),
            (
                Block::Pumpkin {},
                Some(Tool::Axe(ToolMaterial::Wooden)),
                Some(0.75),
            ),
            (
                Block::Pumpkin {},
                Some(Tool::Axe(ToolMaterial::Stone)),
                Some(0.4),
            ),
            (
                Block::Pumpkin {},
                Some(Tool::Axe(ToolMaterial::Iron)),
                Some(0.25),
            ),
            (pumpkin_lit, None, Some(1.5)),
            (
                pumpkin_lit,
                Some(Tool::Axe(ToolMaterial::Wooden)),
                Some(0.75),
            ),
            (pumpkin_lit, Some(Tool::Axe(ToolMaterial::Stone)), Some(0.4)),
            (pumpkin_lit, Some(Tool::Axe(ToolMaterial::Iron)), Some(0.25)),
            // TODO: Fix special sword rules
            //(Block::Web {}, Some(Tool::Sword(ToolMaterial::Wood)), Some(0.4)),
            //(Block::Web {}, Some(Tool::Sword(ToolMaterial::Stone)), Some(0.4)),
            //(cocoa, Some(Tool::Sword(ToolMaterial::Stone)), Some(0.2)),
            //(leaves, Some(Tool::Sword(ToolMaterial::St
            //(leaves2, Some(Tool::Sword(ToolMaterial::Stone)), Some(0.2)),
            //(Block::MelonBlock {}, Some(Tool::Sword(ToolMaterial::Stone)), Some(1.0)),
            //(Block::Pumpkin {}, Some(Tool::Sword(ToolMaterial::Stone)), Some(1.0)),
            //(pumpkin_lit, Some(Tool::Sword(ToolMaterial::Stone)), Some(1.0)),
            //(vine, Some(Tool::Sword(ToolMaterial::Stone)), Some(0.2)),

            // TODO: Fix special shears rules
            //(Block::Web {}, Some(Tool::Shears), Some(0.4)),
            //(wool, Some(Tool::Shears), Some(0.25)),
            //(leaves, Some(Tool::Shears), Some(0.05)),
            //(leaves2, Some(Tool::Shears), Some(0.05)),
            //(vine, Some(Tool::Shears), Some(0.3)),
        ];
        for (block, tool, time) in data {
            let result = block.get_mining_time(&tool).map(|d| d.as_secs_f64());
            match (time, result) {
                (Some(time), Some(result)) => assert_eq!(result, time,
                    "Expected to mine block {:?} with {:?} in {} seconds, but it took {} seconds",
                    block, tool, time, result),
                (None, Some(result)) => panic!(
                    "Expected to never mine block {:?} with {:?}, but it took {} seconds",
                    block, tool, result),
                (Some(time), None) => panic!(
                    "Expected to mine block {:?} with {:?} in {} seconds, but it will never be mined",
                    block, tool, time),
                _ => {},
            }
        }
    }
}

pub enum MiningTime {
    Instant,
    Time(std::time::Duration),
    Never,
}

pub fn get_mining_time(block: &Block, tool: &Option<Tool>) -> MiningTime {
    let mut speed_multiplier = 1.0;

    let tool_multiplier = tool.map(|t| t.get_multiplier()).unwrap_or(1.0);

    let is_best_tool = block.is_best_tool(tool);
    let can_harvest: bool = block.can_harvest(tool);

    if is_best_tool {
        speed_multiplier = tool_multiplier;
    }

    // TODO: Apply tool efficiency
    // TODO: Apply haste effect
    // TODO: Apply mining fatigue effect
    // TODO: Apply in water multiplier
    // TODO: Apply on ground multiplier

    let mut damage = match block.get_hardness() {
        // Instant mine
        Some(n) if n == 0.0 => return MiningTime::Instant,
        // Impossible to mine
        None => return MiningTime::Never,
        Some(n) => speed_multiplier / n,
    };

    if can_harvest {
        damage /= 30.0;
    } else {
        damage /= 100.0;
    }

    // Instant breaking
    if damage > 1.0 {
        return MiningTime::Instant;
    }

    let ticks = (1.0 / damage).ceil();
    let seconds = ticks / 20.0;
    MiningTime::Time(std::time::Duration::from_secs_f64(seconds))
}

fn can_burn<W: WorldAccess>(world: &W, pos: Position) -> bool {
    matches!(
        world.get_block(pos),
        Block::CoalBlock { .. }
            // Planks
            | Block::OakPlanks { .. }
            | Block::SprucePlanks { .. }
            | Block::BirchPlanks { .. }
            | Block::JunglePlanks { .. }
            | Block::AcaciaPlanks { .. }
            | Block::DarkOakPlanks { .. }
            // Logs
            | Block::OakLog { .. }
            | Block::SpruceLog { .. }
            | Block::BirchLog { .. }
            | Block::JungleLog { .. }
            | Block::AcaciaLog { .. }
            | Block::DarkOakLog { .. }
            // Wood
            | Block::OakWood { .. }
            | Block::SpruceWood { .. }
            | Block::BirchWood { .. }
            | Block::JungleWood { .. }
            | Block::AcaciaWood { .. }
            | Block::DarkOakWood { .. }
            // Slabs
            | Block::OakSlab { .. }
            | Block::SpruceSlab { .. }
            | Block::BirchSlab { .. }
            | Block::JungleSlab { .. }
            | Block::AcaciaSlab { .. }
            | Block::DarkOakSlab { .. }
            // Fence gates
            | Block::OakFenceGate { .. }
            | Block::SpruceFenceGate { .. }
            | Block::BirchFenceGate { .. }
            | Block::JungleFenceGate { .. }
            | Block::AcaciaFenceGate { .. }
            | Block::DarkOakFenceGate { .. }
            // Fences
            | Block::OakFence { .. }
            | Block::SpruceFence { .. }
            | Block::BirchFence { .. }
            | Block::JungleFence { .. }
            | Block::AcaciaFence { .. }
            | Block::DarkOakFence { .. }
            // Stairs
            | Block::OakStairs { .. }
            | Block::SpruceStairs { .. }
            | Block::BirchStairs { .. }
            | Block::JungleStairs { .. }
            | Block::AcaciaStairs { .. }
            | Block::DarkOakStairs { .. }
            // Leaves
            | Block::OakLeaves { .. }
            | Block::SpruceLeaves { .. }
            | Block::BirchLeaves { .. }
            | Block::JungleLeaves { .. }
            | Block::AcaciaLeaves { .. }
            | Block::DarkOakLeaves { .. }
            // Wool
            | Block::WhiteWool { .. }
            | Block::OrangeWool { .. }
            | Block::MagentaWool { .. }
            | Block::LightBlueWool { .. }
            | Block::YellowWool { .. }
            | Block::LimeWool { .. }
            | Block::PinkWool { .. }
            | Block::GrayWool { .. }
            | Block::LightGrayWool { .. }
            | Block::CyanWool { .. }
            | Block::PurpleWool { .. }
            | Block::BlueWool { .. }
            | Block::BrownWool { .. }
            | Block::GreenWool { .. }
            | Block::RedWool { .. }
            | Block::BlackWool { .. }
            // Carpet
            | Block::WhiteCarpet { .. }
            | Block::OrangeCarpet { .. }
            | Block::MagentaCarpet { .. }
            | Block::LightBlueCarpet { .. }
            | Block::YellowCarpet { .. }
            | Block::LimeCarpet { .. }
            | Block::PinkCarpet { .. }
            | Block::GrayCarpet { .. }
            | Block::LightGrayCarpet { .. }
            | Block::CyanCarpet { .. }
            | Block::PurpleCarpet { .. }
            | Block::BlueCarpet { .. }
            | Block::BrownCarpet { .. }
            | Block::GreenCarpet { .. }
            | Block::RedCarpet { .. }
            | Block::BlackCarpet { .. }
            // Flowers
            | Block::Dandelion { .. }
            | Block::Poppy { .. }
            | Block::BlueOrchid { .. }
            | Block::Allium { .. }
            | Block::AzureBluet { .. }
            | Block::RedTulip { .. }
            | Block::OrangeTulip { .. }
            | Block::WhiteTulip { .. }
            | Block::PinkTulip { .. }
            | Block::OxeyeDaisy { .. }
            // Tall flower
            | Block::Sunflower { .. }
            | Block::Lilac { .. }
            | Block::TallGrass { .. }
            | Block::LargeFern { .. }
            | Block::RoseBush { .. }
            | Block::Peony { .. }
            // Grass
            | Block::DeadBush { .. }
            | Block::Grass { .. }
            | Block::Fern { .. }
            // Misc
            | Block::Tnt { .. }
            | Block::Vine { .. }
            | Block::Bookshelf { .. }
            | Block::HayBlock { .. }
    )
}

fn is_snowy<W: WorldAccess>(world: &W, pos: Position) -> bool {
    matches!(
        world.get_block(pos.shift(Direction::Up)),
        Block::Snow { .. }
    )
}

fn can_connect_sides<F: Fn(Block) -> bool, W: WorldAccess>(
    world: &W,
    pos: Position,
    f: &F,
) -> (bool, bool, bool, bool) {
    (
        can_connect(world, pos.shift(Direction::North), f),
        can_connect(world, pos.shift(Direction::South), f),
        can_connect(world, pos.shift(Direction::West), f),
        can_connect(world, pos.shift(Direction::East), f),
    )
}

fn can_connect<F: Fn(Block) -> bool, W: WorldAccess>(world: &W, pos: Position, f: &F) -> bool {
    let block = world.get_block(pos);
    f(block) || (block.get_material().renderable && block.get_material().should_cull_against)
}

fn can_connect_fence(block: Block) -> bool {
    matches!(
        block,
        Block::OakFence { .. }
            | Block::SpruceFence { .. }
            | Block::BirchFence { .. }
            | Block::JungleFence { .. }
            | Block::DarkOakFence { .. }
            | Block::AcaciaFence { .. }
            | Block::OakFenceGate { .. }
            | Block::SpruceFenceGate { .. }
            | Block::BirchFenceGate { .. }
            | Block::JungleFenceGate { .. }
            | Block::DarkOakFenceGate { .. }
            | Block::AcaciaFenceGate { .. }
    )
}

fn can_connect_glasspane(block: Block) -> bool {
    matches!(
        block,
        Block::GlassPane { .. }
            | Block::WhiteStainedGlassPane { .. }
            | Block::OrangeStainedGlassPane { .. }
            | Block::MagentaStainedGlassPane { .. }
            | Block::LightBlueStainedGlassPane { .. }
            | Block::YellowStainedGlassPane { .. }
            | Block::LimeStainedGlassPane { .. }
            | Block::PinkStainedGlassPane { .. }
            | Block::GrayStainedGlassPane { .. }
            | Block::LightGrayStainedGlassPane { .. }
            | Block::CyanStainedGlassPane { .. }
            | Block::PurpleStainedGlassPane { .. }
            | Block::BlueStainedGlassPane { .. }
            | Block::BrownStainedGlassPane { .. }
            | Block::GreenStainedGlassPane { .. }
            | Block::RedStainedGlassPane { .. }
            | Block::BlackStainedGlassPane { .. }
    )
}

fn can_connect_redstone<W: WorldAccess>(world: &W, pos: Position, dir: Direction) -> RedstoneSide {
    let shift_pos = pos.shift(dir);
    let block = world.get_block(shift_pos);

    if matches!(
        block,
        RedstoneBlock { .. }
            | OakButton { .. }
            | StoneButton { .. }
            | DaylightDetector { .. }
            | DetectorRail { .. }
            | Lever { .. }
            | Observer { .. }
            | OakPressurePlate { .. }
            | StonePressurePlate { .. }
            | LightWeightedPressurePlate { .. }
            | HeavyWeightedPressurePlate { .. }
            | RedstoneTorch { .. }
            | TrappedChest { .. }
            | TripwireHook { .. }
            | Comparator { .. }
    ) {
        return RedstoneSide::Side;
    }

    if let Repeater { facing, .. } = block {
        if facing == dir || facing.opposite() == dir {
            return RedstoneSide::Side;
        }
        return RedstoneSide::None;
    }

    if block.get_material().should_cull_against {
        let side_up = world.get_block(shift_pos.shift(Direction::Up));
        let up = world.get_block(pos.shift(Direction::Up));

        if matches!(side_up, Block::RedstoneWire { .. }) && !up.get_material().should_cull_against {
            return RedstoneSide::Up;
        }

        return RedstoneSide::None;
    }

    let side_down = world.get_block(shift_pos.shift(Direction::Down));
    if matches!(block, Block::RedstoneWire { .. })
        || matches!(side_down, Block::RedstoneWire { .. })
    {
        return RedstoneSide::Side;
    }
    RedstoneSide::None
}

fn fence_gate_update_state<W: WorldAccess>(world: &W, pos: Position, facing: Direction) -> bool {
    match world.get_block(pos.shift(facing.clockwise())) {
        CobblestoneWall { .. } | MossyCobblestoneWall { .. } => return true,
        _ => {}
    }

    match world.get_block(pos.shift(facing.counter_clockwise())) {
        CobblestoneWall { .. } | MossyCobblestoneWall { .. } => return true,
        _ => {}
    }

    false
}

fn update_redstone_state<W: WorldAccess>(world: &W, pos: Position, power: u8) -> Block {
    let (mut north, mut south, mut west, mut east) = (
        can_connect_redstone(world, pos, Direction::North),
        can_connect_redstone(world, pos, Direction::South),
        can_connect_redstone(world, pos, Direction::West),
        can_connect_redstone(world, pos, Direction::East),
    );

    if north == RedstoneSide::None && south == RedstoneSide::None {
        match (west, east) {
            (RedstoneSide::None, RedstoneSide::None) => {}
            (RedstoneSide::None, _) => west = RedstoneSide::Side,
            (_, RedstoneSide::None) => east = RedstoneSide::Side,
            _ => {}
        }
    }

    if west == RedstoneSide::None && east == RedstoneSide::None {
        match (north, south) {
            (RedstoneSide::None, RedstoneSide::None) => {}
            (RedstoneSide::None, _) => north = RedstoneSide::Side,
            (_, RedstoneSide::None) => south = RedstoneSide::Side,
            _ => {}
        }
    }

    RedstoneWire {
        north,
        south,
        west,
        east,
        power,
    }
}

fn update_fire_state<W: WorldAccess>(world: &W, pos: Position, age: u8) -> Block {
    match world.get_block(pos.shift(Direction::Down)) {
        Air {} => Fire {
            age,
            up: false,
            north: false,
            south: false,
            west: false,
            east: false,
        },
        _ => Fire {
            age,
            up: can_burn(world, pos.shift(Direction::Up)),
            north: can_burn(world, pos.shift(Direction::North)),
            south: can_burn(world, pos.shift(Direction::South)),
            west: can_burn(world, pos.shift(Direction::West)),
            east: can_burn(world, pos.shift(Direction::East)),
        },
    }
}

fn update_door_state<W: WorldAccess>(
    world: &W,
    pos: Position,
    ohalf: DoorHalf,
    ofacing: Direction,
    ohinge: Side,
    oopen: bool,
    opowered: bool,
) -> (Direction, Side, bool, bool) {
    let oy = if ohalf == DoorHalf::Upper { -1 } else { 1 };

    match world.get_block(pos + (0, oy, 0)) {
        Block::OakDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::SpruceDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::BirchDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::JungleDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::AcaciaDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::DarkOakDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        }
        | Block::IronDoor {
            half,
            facing,
            hinge,
            open,
            powered,
        } => {
            if half != ohalf {
                if ohalf == DoorHalf::Upper {
                    return (facing, ohinge, open, opowered);
                } else {
                    return (ofacing, hinge, oopen, powered);
                }
            }
        }
        _ => {}
    }

    (ofacing, ohinge, oopen, opowered)
}

fn update_repeater_state<W: WorldAccess>(world: &W, pos: Position, facing: Direction) -> bool {
    let f = |dir| match world.get_block(pos.shift(dir)) {
        Repeater {
            facing, powered, ..
        }
        | Comparator {
            facing, powered, ..
        } => powered && facing == dir,
        _ => false,
    };

    f(facing.clockwise()) || f(facing.counter_clockwise())
}

fn update_double_plant_state<W: WorldAccess>(world: &W, pos: Position, half: BlockHalf) -> Block {
    if half != BlockHalf::Upper {
        return world.get_block(pos);
    }

    match world.get_block(pos.shift(Direction::Down)) {
        Block::Sunflower { .. } => Block::Sunflower { half },
        Block::Lilac { .. } => Block::Lilac { half },
        Block::TallGrass { .. } => Block::TallGrass { half },
        Block::LargeFern { .. } => Block::LargeFern { half },
        Block::RoseBush { .. } => Block::RoseBush { half },
        Block::Peony { .. } => Block::Peony { half },
        Block::Air {} => world.get_block(pos), // FIXME: is this the correct way to handle air? (if we don't do this 1.8.9 crashes sometimes)
        block => unreachable!("unexpected tall block: {:?}", block),
    }
}

fn get_stair_info<W: WorldAccess>(world: &W, pos: Position) -> Option<(Direction, BlockHalf)> {
    match world.get_block(pos) {
        Block::OakStairs { facing, half, .. }
        | Block::CobblestoneStairs { facing, half, .. }
        | Block::BrickStairs { facing, half, .. }
        | Block::StoneBrickStairs { facing, half, .. }
        | Block::NetherBrickStairs { facing, half, .. }
        | Block::SandstoneStairs { facing, half, .. }
        | Block::SpruceStairs { facing, half, .. }
        | Block::BirchStairs { facing, half, .. }
        | Block::JungleStairs { facing, half, .. }
        | Block::QuartzStairs { facing, half, .. }
        | Block::AcaciaStairs { facing, half, .. }
        | Block::DarkOakStairs { facing, half, .. }
        | Block::RedSandstoneStairs { facing, half, .. }
        | Block::PurpurStairs { facing, half, .. } => Some((facing, half)),
        _ => None,
    }
}

fn update_stair_shape<W: WorldAccess>(world: &W, pos: Position, facing: Direction) -> StairShape {
    if let Some((other_facing, _)) = get_stair_info(world, pos.shift(facing.opposite())) {
        if other_facing == facing.clockwise() {
            if let Some((other_facing, _)) = get_stair_info(world, pos.shift(facing.clockwise())) {
                if facing == other_facing {
                    return StairShape::Straight;
                }
            }

            return StairShape::InnerRight;
        }

        if other_facing == facing.counter_clockwise() {
            if let Some((other_facing, _)) =
                get_stair_info(world, pos.shift(facing.counter_clockwise()))
            {
                if facing == other_facing {
                    return StairShape::Straight;
                }
            }

            return StairShape::InnerLeft;
        }
    }

    if let Some((other_facing, _)) = get_stair_info(world, pos.shift(facing)) {
        if other_facing == facing.clockwise() {
            if let Some((other_facing, _)) =
                get_stair_info(world, pos.shift(facing.counter_clockwise()))
            {
                if facing == other_facing {
                    return StairShape::Straight;
                }
            }

            return StairShape::OuterRight;
        }

        if other_facing == facing.counter_clockwise() {
            if let Some((other_facing, _)) = get_stair_info(world, pos.shift(facing.clockwise())) {
                if facing == other_facing {
                    return StairShape::Straight;
                }
            }

            return StairShape::OuterLeft;
        }
    }

    StairShape::Straight
}

fn update_wall_state<W: WorldAccess>(
    world: &W,
    pos: Position,
) -> (bool, WallSide, WallSide, WallSide, WallSide) {
    let f = |block| {
        matches!(
            block,
            CobblestoneWall { .. }
                | MossyCobblestoneWall { .. }
                | OakFenceGate { .. }
                | SpruceFenceGate { .. }
                | BirchFenceGate { .. }
                | JungleFenceGate { .. }
                | DarkOakFenceGate { .. }
                | AcaciaFenceGate { .. }
        )
    };

    let (north, south, west, east) = can_connect_sides(world, pos, &f);

    #[allow(clippy::nonminimal_bool)]
    let up = !matches!(world.get_block(pos.shift(Direction::Up)), Air {})
        || !((north && south && !west && !east) || (!north && !south && west && east));

    let north = if north { WallSide::Low } else { WallSide::None };
    let south = if south { WallSide::Low } else { WallSide::None };
    let west = if west { WallSide::Low } else { WallSide::None };
    let east = if east { WallSide::Low } else { WallSide::None };
    (up, north, south, west, east)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoneVariant {
    Normal,
    Granite,
    SmoothGranite,
    Diorite,
    SmoothDiorite,
    Andesite,
    SmoothAndesite,
}

impl StoneVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            StoneVariant::Normal => "stone",
            StoneVariant::Granite => "granite",
            StoneVariant::SmoothGranite => "polished_granite",
            StoneVariant::Diorite => "diorite",
            StoneVariant::SmoothDiorite => "polished_diorite",
            StoneVariant::Andesite => "andesite",
            StoneVariant::SmoothAndesite => "polished_andesite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DirtVariant {
    Normal,
    Coarse,
    Podzol,
}

impl DirtVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            DirtVariant::Normal => "dirt",
            DirtVariant::Coarse => "coarse_dirt",
            DirtVariant::Podzol => "podzol",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BedPart {
    Head,
    Foot,
}

impl BedPart {
    pub fn as_string(self) -> &'static str {
        match self {
            BedPart::Head => "head",
            BedPart::Foot => "foot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SandstoneVariant {
    Normal,
    Chiseled,
    Smooth,
}

impl SandstoneVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            SandstoneVariant::Normal => "sandstone",
            SandstoneVariant::Chiseled => "chiseled_sandstone",
            SandstoneVariant::Smooth => "smooth_sandstone",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoteBlockInstrument {
    Harp,
    Basedrum,
    Snare,
    Hat,
    Bass,
    Flute,
    Bell,
    Guitar,
    Chime,
    Xylophone,
    IronXylophone,
    CowBell,
    Didgeridoo,
    Bit,
    Banjo,
    Pling,
}

impl NoteBlockInstrument {
    pub fn as_string(self) -> &'static str {
        match self {
            NoteBlockInstrument::Harp => "harp",
            NoteBlockInstrument::Basedrum => "basedrum",
            NoteBlockInstrument::Snare => "snare",
            NoteBlockInstrument::Hat => "hat",
            NoteBlockInstrument::Bass => "bass",
            NoteBlockInstrument::Flute => "flute",
            NoteBlockInstrument::Bell => "bell",
            NoteBlockInstrument::Guitar => "guitar",
            NoteBlockInstrument::Chime => "chime",
            NoteBlockInstrument::Xylophone => "xylophone",
            NoteBlockInstrument::IronXylophone => "iron_xylophone",
            NoteBlockInstrument::CowBell => "cow_bell",
            NoteBlockInstrument::Didgeridoo => "didgeridoo",
            NoteBlockInstrument::Bit => "bit",
            NoteBlockInstrument::Banjo => "banjo",
            NoteBlockInstrument::Pling => "pling",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RedSandstoneVariant {
    Normal,
    Chiseled,
    Smooth,
}

impl RedSandstoneVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            RedSandstoneVariant::Normal => "red_sandstone",
            RedSandstoneVariant::Chiseled => "chiseled_red_sandstone",
            RedSandstoneVariant::Smooth => "smooth_red_sandstone",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuartzVariant {
    Normal,
    Chiseled,
    PillarVertical,
    PillarNorthSouth,
    PillarEastWest,
}

impl QuartzVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            QuartzVariant::Normal | QuartzVariant::Chiseled => "normal",
            QuartzVariant::PillarVertical => "axis=y",
            QuartzVariant::PillarNorthSouth => "axis=z",
            QuartzVariant::PillarEastWest => "axis=x",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrismarineVariant {
    Normal,
    Brick,
    Dark,
}

impl PrismarineVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            PrismarineVariant::Normal => "prismarine",
            PrismarineVariant::Brick => "prismarine_bricks",
            PrismarineVariant::Dark => "dark_prismarine",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DoorHalf {
    Upper,
    Lower,
}

impl DoorHalf {
    pub fn as_string(self) -> &'static str {
        match self {
            DoorHalf::Upper => "upper",
            DoorHalf::Lower => "lower",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn as_string(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColoredVariant {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl ColoredVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            ColoredVariant::White => "white",
            ColoredVariant::Orange => "orange",
            ColoredVariant::Magenta => "magenta",
            ColoredVariant::LightBlue => "light_blue",
            ColoredVariant::Yellow => "yellow",
            ColoredVariant::Lime => "lime",
            ColoredVariant::Pink => "pink",
            ColoredVariant::Gray => "gray",
            ColoredVariant::LightGray => "light_gray",
            ColoredVariant::Cyan => "cyan",
            ColoredVariant::Purple => "purple",
            ColoredVariant::Blue => "blue",
            ColoredVariant::Brown => "brown",
            ColoredVariant::Green => "green",
            ColoredVariant::Red => "red",
            ColoredVariant::Black => "black",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RedFlowerVariant {
    Poppy,
    BlueOrchid,
    Allium,
    AzureBluet,
    RedTulip,
    OrangeTulip,
    WhiteTulip,
    PinkTulip,
    OxeyeDaisy,
    Cornflower,
    WitherRose,
    LilyOfTheValley,
}

impl RedFlowerVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            RedFlowerVariant::Poppy => "poppy",
            RedFlowerVariant::BlueOrchid => "blue_orchid",
            RedFlowerVariant::Allium => "allium",
            RedFlowerVariant::AzureBluet => "houstonia",
            RedFlowerVariant::RedTulip => "red_tulip",
            RedFlowerVariant::OrangeTulip => "orange_tulip",
            RedFlowerVariant::WhiteTulip => "white_tulip",
            RedFlowerVariant::PinkTulip => "pink_tulip",
            RedFlowerVariant::OxeyeDaisy => "oxeye_daisy",
            RedFlowerVariant::Cornflower => "cornflower",
            RedFlowerVariant::WitherRose => "wither_rose",
            RedFlowerVariant::LilyOfTheValley => "lily_of_the_valley",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonsterEggVariant {
    Stone,
    Cobblestone,
    StoneBrick,
    MossyBrick,
    CrackedBrick,
    ChiseledBrick,
}

impl MonsterEggVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            MonsterEggVariant::Stone => "stone",
            MonsterEggVariant::Cobblestone => "cobblestone",
            MonsterEggVariant::StoneBrick => "stone_brick",
            MonsterEggVariant::MossyBrick => "mossy_brick",
            MonsterEggVariant::CrackedBrick => "cracked_brick",
            MonsterEggVariant::ChiseledBrick => "chiseled_brick",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoneBrickVariant {
    Normal,
    Mossy,
    Cracked,
    Chiseled,
}

impl StoneBrickVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            StoneBrickVariant::Normal => "stonebrick",
            StoneBrickVariant::Mossy => "mossy_stonebrick",
            StoneBrickVariant::Cracked => "cracked_stonebrick",
            StoneBrickVariant::Chiseled => "chiseled_stonebrick",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RailShape {
    NorthSouth,
    EastWest,
    AscendingNorth,
    AscendingSouth,
    AscendingEast,
    AscendingWest,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl RailShape {
    pub fn as_string(self) -> &'static str {
        match self {
            RailShape::NorthSouth => "north_south",
            RailShape::EastWest => "east_west",
            RailShape::AscendingNorth => "ascending_north",
            RailShape::AscendingSouth => "ascending_south",
            RailShape::AscendingEast => "ascending_east",
            RailShape::AscendingWest => "ascending_west",
            RailShape::NorthEast => "north_east",
            RailShape::NorthWest => "north_west",
            RailShape::SouthEast => "south_east",
            RailShape::SouthWest => "south_west",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComparatorMode {
    Compare,
    Subtract,
}

impl ComparatorMode {
    pub fn as_string(self) -> &'static str {
        match self {
            ComparatorMode::Compare => "compare",
            ComparatorMode::Subtract => "subtract",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RedstoneSide {
    None,
    Side,
    Up,
}

impl RedstoneSide {
    pub fn as_string(self) -> &'static str {
        match self {
            RedstoneSide::None => "none",
            RedstoneSide::Side => "side",
            RedstoneSide::Up => "up",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PistonType {
    Normal,
    Sticky,
}

impl PistonType {
    pub fn as_string(self) -> &'static str {
        match self {
            PistonType::Normal => "normal",
            PistonType::Sticky => "sticky",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoneSlabVariant {
    Stone,
    SmoothStone,
    Sandstone,
    CutSandstone,
    PetrifiedWood,
    Cobblestone,
    Brick,
    StoneBrick,
    NetherBrick,
    Quartz,
    RedSandstone,
    CutRedSandstone,
    Purpur,
}

impl StoneSlabVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            StoneSlabVariant::Stone => "stone",
            StoneSlabVariant::SmoothStone => "smooth_stone",
            StoneSlabVariant::Sandstone => "sandstone",
            StoneSlabVariant::CutSandstone => "cut_sandstone",
            StoneSlabVariant::PetrifiedWood => "wood_old",
            StoneSlabVariant::Cobblestone => "cobblestone",
            StoneSlabVariant::Brick => "brick",
            StoneSlabVariant::StoneBrick => "stone_brick",
            StoneSlabVariant::NetherBrick => "nether_bricks",
            StoneSlabVariant::Quartz => "quartz",
            StoneSlabVariant::RedSandstone => "red_sandstone",
            StoneSlabVariant::CutRedSandstone => "cut_red_sandstone",
            StoneSlabVariant::Purpur => "purpur",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WoodSlabVariant {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
}

impl WoodSlabVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            WoodSlabVariant::Oak => "oak",
            WoodSlabVariant::Spruce => "spruce",
            WoodSlabVariant::Birch => "birch",
            WoodSlabVariant::Jungle => "jungle",
            WoodSlabVariant::Acacia => "acacia",
            WoodSlabVariant::DarkOak => "dark_oak",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockHalf {
    Top,
    Bottom,
    Upper,
    Lower,
    Double,
}

impl BlockHalf {
    pub fn as_string(self) -> &'static str {
        match self {
            BlockHalf::Top => "top",
            BlockHalf::Bottom => "bottom",
            BlockHalf::Upper => "upper",
            BlockHalf::Lower => "lower",
            BlockHalf::Double => "double",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CobblestoneWallVariant {
    Normal,
    Mossy,
}

impl CobblestoneWallVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            CobblestoneWallVariant::Normal => "cobblestone",
            CobblestoneWallVariant::Mossy => "mossy_cobblestone",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rotation {
    South,
    SouthSouthWest,
    SouthWest,
    WestSouthWest,
    West,
    WestNorthWest,
    NorthWest,
    NorthNorthWest,
    North,
    NorthNorthEast,
    NorthEast,
    EastNorthEast,
    East,
    EastSouthEast,
    SouthEast,
    SouthSouthEast,
}

impl Rotation {
    pub fn as_string(self) -> &'static str {
        match self {
            Rotation::South => "south",
            Rotation::SouthSouthWest => "south-southwest",
            Rotation::SouthWest => "southwest",
            Rotation::WestSouthWest => "west-southwest",
            Rotation::West => "west",
            Rotation::WestNorthWest => "west-northwest",
            Rotation::NorthWest => "northwest",
            Rotation::NorthNorthWest => "north-northwest",
            Rotation::North => "north",
            Rotation::NorthNorthEast => "north-northeast",
            Rotation::NorthEast => "northeast",
            Rotation::EastNorthEast => "east-northeast",
            Rotation::East => "east",
            Rotation::EastSouthEast => "east-southeast",
            Rotation::SouthEast => "southseast",
            Rotation::SouthSouthEast => "south-southeast",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StairShape {
    Straight,
    InnerLeft,
    InnerRight,
    OuterLeft,
    OuterRight,
}

impl StairShape {
    pub fn as_string(self) -> &'static str {
        match self {
            StairShape::Straight => "straight",
            StairShape::InnerLeft => "inner_left",
            StairShape::InnerRight => "inner_right",
            StairShape::OuterLeft => "outer_left",
            StairShape::OuterRight => "outer_right",
        }
    }

    pub fn offset(self) -> usize {
        match self {
            StairShape::Straight => 0,
            StairShape::InnerLeft => 1,
            StairShape::InnerRight => 2,
            StairShape::OuterLeft => 3,
            StairShape::OuterRight => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachedFace {
    Floor,
    Wall,
    Ceiling,
}

impl AttachedFace {
    pub fn as_string(self) -> &'static str {
        match self {
            AttachedFace::Floor => "floor",
            AttachedFace::Wall => "wall",
            AttachedFace::Ceiling => "ceiling",
        }
    }

    pub fn data_with_facing(self, facing: Direction) -> Option<usize> {
        Some(match (self, facing) {
            (AttachedFace::Ceiling, Direction::East) => 0,
            (AttachedFace::Wall, Direction::East) => 1,
            (AttachedFace::Wall, Direction::West) => 2,
            (AttachedFace::Wall, Direction::South) => 3,
            (AttachedFace::Wall, Direction::North) => 4,
            (AttachedFace::Floor, Direction::South) => 5,
            (AttachedFace::Floor, Direction::East) => 6,
            (AttachedFace::Ceiling, Direction::South) => 7,
            _ => return None,
        })
    }

    pub fn data_with_facing_and_powered(self, facing: Direction, powered: bool) -> Option<usize> {
        self.data_with_facing(facing)
            .map(|facing_data| facing_data | if powered { 0x8 } else { 0x0 })
    }

    pub fn variant_with_facing(self, facing: Direction) -> String {
        match (self, facing) {
            (AttachedFace::Ceiling, Direction::East) => "down_x",
            (AttachedFace::Wall, Direction::East) => "east",
            (AttachedFace::Wall, Direction::West) => "west",
            (AttachedFace::Wall, Direction::South) => "south",
            (AttachedFace::Wall, Direction::North) => "north",
            (AttachedFace::Floor, Direction::South) => "up_z",
            (AttachedFace::Floor, Direction::East) => "up_x",
            (AttachedFace::Ceiling, Direction::South) => "down_z",
            _ => "north", // TODO: support 1.13.2+ new directions
        }
        .to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChestType {
    Single,
    Left,
    Right,
}

impl ChestType {
    pub fn as_string(self) -> &'static str {
        match self {
            ChestType::Single => "single",
            ChestType::Left => "left",
            ChestType::Right => "right",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
}

impl StructureBlockMode {
    pub fn as_string(self) -> &'static str {
        match self {
            StructureBlockMode::Save => "save",
            StructureBlockMode::Load => "load",
            StructureBlockMode::Corner => "corner",
            StructureBlockMode::Data => "data",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeVariant {
    Oak,
    Spruce,
    Birch,
    Jungle,
    Acacia,
    DarkOak,
    StrippedSpruce,
    StrippedBirch,
    StrippedJungle,
    StrippedAcacia,
    StrippedDarkOak,
    StrippedOak,
}

impl TreeVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            TreeVariant::Oak => "oak",
            TreeVariant::Spruce => "spruce",
            TreeVariant::Birch => "birch",
            TreeVariant::Jungle => "jungle",
            TreeVariant::Acacia => "acacia",
            TreeVariant::DarkOak => "dark_oak",
            TreeVariant::StrippedSpruce => "stripped_spruce_log",
            TreeVariant::StrippedBirch => "stripped_birch_log",
            TreeVariant::StrippedJungle => "stripped_jungle_log",
            TreeVariant::StrippedAcacia => "stripped_acacia_log",
            TreeVariant::StrippedDarkOak => "stripped_dark_oak_log",
            TreeVariant::StrippedOak => "stripped_oak_log",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TallGrassVariant {
    DeadBush,
    TallGrass,
    Fern,
}

impl TallGrassVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            TallGrassVariant::DeadBush => "dead_bush",
            TallGrassVariant::TallGrass => "tall_grass",
            TallGrassVariant::Fern => "fern",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TallSeagrassHalf {
    Upper,
    Lower,
}

impl TallSeagrassHalf {
    pub fn as_string(self) -> &'static str {
        match self {
            TallSeagrassHalf::Upper => "upper",
            TallSeagrassHalf::Lower => "lower",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DoublePlantVariant {
    Sunflower,
    Lilac,
    DoubleTallgrass,
    LargeFern,
    RoseBush,
    Peony,
}

impl DoublePlantVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            DoublePlantVariant::Sunflower => "sunflower",
            DoublePlantVariant::Lilac => "syringa",
            DoublePlantVariant::DoubleTallgrass => "double_grass",
            DoublePlantVariant::LargeFern => "double_fern",
            DoublePlantVariant::RoseBush => "double_rose",
            DoublePlantVariant::Peony => "paeonia",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowerPotVariant {
    Empty,
    Poppy,
    Dandelion,
    OakSapling,
    SpruceSapling,
    BirchSapling,
    JungleSapling,
    RedMushroom,
    BrownMushroom,
    Cactus,
    DeadBush,
    Fern,
    AcaciaSapling,
    DarkOakSapling,
    BlueOrchid,
    Allium,
    AzureBluet,
    RedTulip,
    OrangeTulip,
    WhiteTulip,
    PinkTulip,
    Oxeye,
    Cornflower,
    LilyOfTheValley,
    WitherRose,
}

impl FlowerPotVariant {
    pub fn as_string(self) -> &'static str {
        match self {
            FlowerPotVariant::Empty => "empty",
            FlowerPotVariant::Poppy => "rose",
            FlowerPotVariant::Dandelion => "dandelion",
            FlowerPotVariant::OakSapling => "oak_sapling",
            FlowerPotVariant::SpruceSapling => "spruce_sapling",
            FlowerPotVariant::BirchSapling => "birch_sapling",
            FlowerPotVariant::JungleSapling => "jungle_sapling",
            FlowerPotVariant::RedMushroom => "mushroom_red",
            FlowerPotVariant::BrownMushroom => "mushroom_brown",
            FlowerPotVariant::Cactus => "cactus",
            FlowerPotVariant::DeadBush => "dead_bush",
            FlowerPotVariant::Fern => "fern",
            FlowerPotVariant::AcaciaSapling => "acacia_sapling",
            FlowerPotVariant::DarkOakSapling => "dark_oak_sapling",
            FlowerPotVariant::BlueOrchid => "blue_orchid",
            FlowerPotVariant::Allium => "allium",
            FlowerPotVariant::AzureBluet => "houstonia",
            FlowerPotVariant::RedTulip => "red_tulip",
            FlowerPotVariant::OrangeTulip => "orange_tulip",
            FlowerPotVariant::WhiteTulip => "white_tulip",
            FlowerPotVariant::PinkTulip => "pink_tulip",
            FlowerPotVariant::Oxeye => "oxeye_daisy",
            FlowerPotVariant::Cornflower => "cornflower",
            FlowerPotVariant::LilyOfTheValley => "lily_of_the_valley",
            FlowerPotVariant::WitherRose => "wither_rose",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WallSide {
    None,
    Low,
    Tall,
}

impl WallSide {
    pub fn as_string(self) -> &'static str {
        match self {
            WallSide::None => "none",
            WallSide::Low => "low",
            WallSide::Tall => "tall",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BambooLeaves {
    None,
    Small,
    Large,
}

impl BambooLeaves {
    pub fn as_string(self) -> &'static str {
        match self {
            BambooLeaves::None => "none",
            BambooLeaves::Small => "small",
            BambooLeaves::Large => "large",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BellAttachment {
    Floor,
    Ceiling,
    SingleWall,
    DoubleWall,
}

impl BellAttachment {
    pub fn as_string(self) -> &'static str {
        match self {
            BellAttachment::Floor => "floor",
            BellAttachment::Ceiling => "ceiling",
            BellAttachment::SingleWall => "single_wall",
            BellAttachment::DoubleWall => "double_wall",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JigsawOrientation {
    DownEast,
    DownNorth,
    DownSouth,
    DownWest,
    UpEast,
    UpNorth,
    UpSouth,
    UpWest,
    WestUp,
    EastUp,
    NorthUp,
    SouthUp,
}

impl JigsawOrientation {
    pub fn as_string(self) -> &'static str {
        match self {
            JigsawOrientation::DownEast => "down_east",
            JigsawOrientation::DownNorth => "down_north",
            JigsawOrientation::DownSouth => "down_south",
            JigsawOrientation::DownWest => "down_west",
            JigsawOrientation::UpEast => "up_east",
            JigsawOrientation::UpNorth => "up_north",
            JigsawOrientation::UpSouth => "up_south",
            JigsawOrientation::UpWest => "up_west",
            JigsawOrientation::WestUp => "west_up",
            JigsawOrientation::EastUp => "east_up",
            JigsawOrientation::NorthUp => "north_up",
            JigsawOrientation::SouthUp => "south_up",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SculkSensorPhase {
    Inactive,
    Active,
    Cooldown,
}

impl SculkSensorPhase {
    pub fn as_string(self) -> &'static str {
        match self {
            SculkSensorPhase::Inactive => "inactive",
            SculkSensorPhase::Active => "active",
            SculkSensorPhase::Cooldown => "cooldown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DripstoneThickness {
    TipMerge,
    Tip,
    Frustum,
    Middle,
    Base,
}

impl DripstoneThickness {
    pub fn as_string(self) -> &'static str {
        match self {
            DripstoneThickness::TipMerge => "tip_merge",
            DripstoneThickness::Tip => "tip",
            DripstoneThickness::Frustum => "frustum",
            DripstoneThickness::Middle => "middle",
            DripstoneThickness::Base => "base",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DripleafTilt {
    None,
    Unstable,
    Partial,
    Full,
}

impl DripleafTilt {
    pub fn as_string(self) -> &'static str {
        match self {
            DripleafTilt::None => "none",
            DripleafTilt::Unstable => "unstable",
            DripleafTilt::Partial => "partial",
            DripleafTilt::Full => "full",
        }
    }
}

impl std::fmt::Display for DripleafTilt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ToolMaterial {
    Wooden,
    Stone,
    Golden,
    Iron,
    Diamond,
    Netherite,
}

impl ToolMaterial {
    fn get_multiplier(&self) -> f64 {
        match *self {
            ToolMaterial::Wooden => 2.0,
            ToolMaterial::Stone => 4.0,
            ToolMaterial::Golden => 12.0,
            ToolMaterial::Iron => 6.0,
            ToolMaterial::Diamond => 8.0,
            ToolMaterial::Netherite => 9.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Tool {
    Pickaxe(ToolMaterial),
    Axe(ToolMaterial),
    Shovel(ToolMaterial),
    Hoe(ToolMaterial),
    Sword(ToolMaterial),
    Shears,
}

impl Tool {
    fn get_multiplier(&self) -> f64 {
        match *self {
            Tool::Pickaxe(m) => m.get_multiplier(),
            Tool::Axe(m) => m.get_multiplier(),
            Tool::Shovel(m) => m.get_multiplier(),
            Tool::Hoe(m) => m.get_multiplier(),
            Tool::Sword(_) => 1.5, // TODO: Handle different block values.
            Tool::Shears => 2.0,   // TODO: Handle different block values.
        }
    }
}
