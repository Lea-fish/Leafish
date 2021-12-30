// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
        let mut e = m.world.spawn();
        e.insert(pos);
        let e = e.id();
        match *self {
            BlockEntityType::Sign => sign::init_entity(m, e),
        }
        e
    }
}
