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

use std::cmp::Ordering;
use std::io::Cursor;
use std::sync::Arc;

use lazy_static::lazy_static;
pub use leafish_blocks as block;

use crate::format;
use crate::shared::Position;
use crate::types::nibble;
use crate::world::biome::Biome;
use crate::world::chunk::chunk_section::SectionSnapshot;

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

// TODO: make use of "x: i32", "y: i32" and "z: i32"
#[allow(dead_code)]
pub struct ComposedSection {
    sections: [Option<SectionSnapshot>; 27],
    x: i32,
    y: i32,
    z: i32,
}

impl ComposedSection {
    // NOTE: This only supports up to 15 blocks in expansion
    pub fn new(world: Arc<World>, x: i32, z: i32, y: i32, expand_by: u8) -> Self {
        let chunk_lookup = world.chunks.clone();
        let mut sections = [
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        ];
        for xo in -1..2 {
            for zo in -1..2 {
                let chunk = chunk_lookup.get(&CPos(x + xo, z + zo));
                let chunk = chunk.as_ref();
                for yo in -1..2 {
                    let section = if let Some(chunk) = chunk {
                        if y + yo != (y + yo) & 15 {
                            None
                        } else {
                            let section = &chunk.sections[(y + yo) as usize].as_ref();
                            if let Some(section) = section {
                                Some(section.capture_snapshot(chunk.biomes))
                            } else {
                                Some(EMPTY_SECTION.clone())
                            }
                        }
                    } else {
                        None
                    };
                    sections[((xo + 1) + (zo + 1) * 3 + (yo + 1) * 3 * 3) as usize] = section;
                }
            }
        }
        ComposedSection {
            sections,
            x: -(expand_by as i32),
            y: -(expand_by as i32),
            z: -(expand_by as i32),
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        let chunk_x = ComposedSection::cmp(x & !15, 0);
        let chunk_z = ComposedSection::cmp(z & !15, 0);
        let chunk_y = ComposedSection::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(block::Missing {}, |s| s.get_block(x, y, z))
    }

    pub fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let chunk_x = ComposedSection::cmp(x & !15, 0);
        let chunk_z = ComposedSection::cmp(z & !15, 0);
        let chunk_y = ComposedSection::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(16, |s| s.get_block_light(x, y, z))
    }

    pub fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let chunk_x = ComposedSection::cmp(x & !15, 0);
        let chunk_z = ComposedSection::cmp(z & !15, 0);
        let chunk_y = ComposedSection::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(16, |s| s.get_sky_light(x, y, z))
    }

    pub fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        let chunk_x = ComposedSection::cmp(x & !15, 0);
        let chunk_z = ComposedSection::cmp(z & !15, 0);
        let section = self.sections[((chunk_x + 1) + (chunk_z + 1) * 3) as usize].as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(Biome::by_id(0), |s| s.get_biome(x, z))
    }

    #[inline]
    fn cmp(first: i32, second: i32) -> i32 {
        // copied from rust's ordering enum's src code
        // The order here is important to generate more optimal assembly.
        match first.cmp(&second) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}
