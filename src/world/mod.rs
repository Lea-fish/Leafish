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

use lazy_static::lazy_static;
pub use leafish_blocks as block;

use crate::format;
use crate::shared::Position;
use crate::types::nibble;
use crate::world::chunk::ChunkSectionSnapshot;

pub use self::{chunk::*, lighting::*, world::*};

pub mod biome;
mod chunk;
mod lighting;
mod storage;
mod world;

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

lazy_static! {
    static ref EMPTY_SECTION: ChunkSectionSnapshot = ChunkSectionSnapshot {
        y: 255, // TODO: Check
        blocks: storage::BlockStorage::new(16 * 16 * 16),
        block_light: nibble::Array::new(16 * 16 * 16),
        sky_light: nibble::Array::new_def(16 * 16 * 16, 0xF),
        biomes: [0; 16 * 16], // TODO: Verify this!
    };
}
