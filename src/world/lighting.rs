// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::Cursor;

use leafish_shared::position::Position;

use crate::world::World;

pub struct LightData {
    pub arrays: Cursor<Vec<u8>>,
    pub block_light_mask: i64,
    pub sky_light_mask: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Block,
    Sky,
}

// TODO: make use of "get_light" and "set_light"
impl LightType {
    #[allow(dead_code)]
    pub(crate) fn get_light(self, world: &World, pos: Position) -> u8 {
        match self {
            LightType::Block => world.get_block_light(pos),
            LightType::Sky => world.get_sky_light(pos),
        }
    }
    #[allow(dead_code)]
    pub(crate) fn set_light(self, world: &World, pos: Position, light: u8) {
        match self {
            LightType::Block => world.set_block_light(pos, light),
            LightType::Sky => world.set_sky_light(pos, light),
        }
    }
}

// TODO: make use of "ty: LightType" and "pos: Position"
#[allow(dead_code)]
pub struct LightUpdate {
    pub(crate) ty: LightType,
    pub(crate) pos: Position,
}
