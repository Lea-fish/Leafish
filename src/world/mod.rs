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

use std::io::Cursor;

use lazy_static::lazy_static;
pub use leafish_blocks as block;

use crate::format;
use crate::shared::Position;
use crate::types::nibble;
use crate::world::chunk::SectionSnapshot;

pub use self::{chunk::*, world::*};

pub mod biome;
mod chunk;
mod storage;
mod world;

pub struct LightData {
    pub arrays: Cursor<Vec<u8>>,
    pub block_light_mask: i32,
    pub sky_light_mask: i32,
}

#[derive(Clone, Debug)]
pub enum BlockEntityAction {
    Create(Position),
    Remove(Position),
    UpdateSignText(
        Box<(
            Position,
            format::Component,
            format::Component,
            format::Component,
            format::Component,
        )>,
    ),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LightType {
    Block,
    Sky,
}

// TODO: make use of "get_light" and "set_light"
impl LightType {
    #[allow(dead_code)]
    fn get_light(self, world: &World, pos: Position) -> u8 {
        match self {
            LightType::Block => world.get_block_light(pos),
            LightType::Sky => world.get_sky_light(pos),
        }
    }
    #[allow(dead_code)]
    fn set_light(self, world: &World, pos: Position, light: u8) {
        match self {
            LightType::Block => world.set_block_light(pos, light),
            LightType::Sky => world.set_sky_light(pos, light),
        }
    }
}

// TODO: make use of "ty: LightType" and "pos: Position"
#[allow(dead_code)]
pub struct LightUpdate {
    ty: LightType,
    pos: Position,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct CPos(pub i32, pub i32);

lazy_static! {
    static ref EMPTY_SECTION: SectionSnapshot = SectionSnapshot {
        y: 255, // TODO: Check
        blocks: storage::BlockStorage::new(16 * 16 * 16),
        block_light: nibble::Array::new(16 * 16 * 16),
        sky_light: nibble::Array::new_def(16 * 16 * 16, 0xF),
        biomes: [0; 16 * 16], // TODO: Verify this!
    };
}
