pub mod sign;

use crate::ecs;
use crate::shared::Position;
use crate::world::block::Block;
use bevy_ecs::prelude::*;

pub fn add_systems(m: &mut ecs::Manager, parallel: &mut SystemStage, sync: &mut SystemStage) {
    sign::add_systems(m, parallel, sync);
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

    pub fn create_entity(&self, m: &mut ecs::Manager, pos: Position) -> Entity {
        let mut e = m.world.spawn();
        e.insert(pos);
        let e = e.id();
        match *self {
            BlockEntityType::Sign => sign::init_entity(m, e),
        }
        e
    }
}
