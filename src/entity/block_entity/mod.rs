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
            Block::StandingSign { .. } | Block::WallSign { .. } => Some(BlockEntityType::Sign),
            _ => None,
        }
    }

    pub fn create_entity(&self, m: &mut ecs::Manager, pos: Position) -> Entity {
        let e = m.create_entity();
        m.add_component_direct(e, pos);
        match *self {
            BlockEntityType::Sign => sign::init_entity(m, e),
        }
        e
    }
}
