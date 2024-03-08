pub mod sign;

use crate::shared::Position;
use crate::world::block::Block;
use bevy_ecs::prelude::*;

pub fn add_systems(sched: &mut Schedule) {
    sign::add_systems(sched);
}

pub enum BlockEntityType {
    Sign,
}

impl BlockEntityType {
    pub fn get_block_entity(bl: Block) -> Option<BlockEntityType> {
        match bl {
            Block::OakSign { .. }
            | Block::SpruceSign { .. }
            | Block::BirchSign { .. }
            | Block::AcaciaSign { .. }
            | Block::JungleSign { .. }
            | Block::DarkOakSign { .. }
            | Block::MangroveSign { .. }
            | Block::CrimsonSign { .. }
            | Block::WarpedSign { .. }
            | Block::OakWallSign { .. }
            | Block::SpruceWallSign { .. }
            | Block::BirchWallSign { .. }
            | Block::AcaciaWallSign { .. }
            | Block::JungleWallSign { .. }
            | Block::DarkOakWallSign { .. }
            | Block::MangroveWallSign { .. }
            | Block::CrimsonWallSign { .. }
            | Block::WarpedWallSign { .. } => Some(BlockEntityType::Sign),
            _ => None,
        }
    }

    pub fn create_entity(&self, cmds: &mut Commands, pos: Position) -> Entity {
        let mut e = cmds.spawn_empty();
        e.insert(pos);
        let e = e.id();
        match *self {
            BlockEntityType::Sign => sign::init_entity(cmds, e),
        }
        e
    }
}
