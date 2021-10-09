use crate::world::biome::Biome;
use crate::world::EMPTY_SECTION;
use crate::world::{biome, storage, CPos, World};
use crate::{chunk_builder, render};
pub use leafish_blocks as block;
use leafish_protocol::types::nibble;
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::sync::Arc;

pub struct ChunkSection {
    pub cull_info: chunk_builder::CullInfo,
    pub render_buffer: Arc<RwLock<render::ChunkBuffer>>,

    pub(crate) y: u8,

    pub(crate) blocks: storage::BlockStorage,

    pub(crate) block_light: nibble::Array,
    pub(crate) sky_light: nibble::Array,

    pub(crate) dirty: bool,
    pub(crate) building: bool,
}

impl ChunkSection {
    pub(crate) fn new(y: u8, fill_sky: bool) -> Self {
        let sky_light = if fill_sky {
            nibble::Array::new_def(16 * 16 * 16, 0xF)
        } else {
            nibble::Array::new(16 * 16 * 16)
        };
        Self {
            cull_info: chunk_builder::CullInfo::all_vis(),
            render_buffer: Arc::new(RwLock::new(render::ChunkBuffer::new())),
            y,

            blocks: storage::BlockStorage::new(16 * 16 * 16),

            block_light: nibble::Array::new(16 * 16 * 16),
            sky_light,

            dirty: false,
            building: false,
        }
    }

    pub fn capture_snapshot(&self, biomes: [u8; 16 * 16]) -> ChunkSectionSnapshot {
        ChunkSectionSnapshot {
            y: self.y,
            blocks: self.blocks.clone(),
            block_light: self.block_light.clone(),
            sky_light: self.sky_light.clone(),
            biomes,
        }
    }

    pub fn blocks_mut(&mut self) -> &mut storage::BlockStorage {
        &mut self.blocks
    }

    pub(crate) fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        self.blocks.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub(crate) fn set_block(&mut self, x: i32, y: i32, z: i32, b: block::Block) -> bool {
        if self.blocks.set(((y << 8) | (z << 4) | x) as usize, b) {
            self.dirty = true;
            self.set_sky_light(x, y, z, 0); // TODO: Do we have to set this every time?
            self.set_block_light(x, y, z, 0);
            true
        } else {
            false
        }
    }

    pub(crate) fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.block_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub(crate) fn set_block_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.block_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }

    pub(crate) fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.sky_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub(crate) fn set_sky_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.sky_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }
}

#[derive(Clone)]
pub struct ChunkSectionSnapshot {
    pub y: u8,
    pub blocks: storage::BlockStorage,
    pub block_light: nibble::Array,
    pub sky_light: nibble::Array,
    pub biomes: [u8; 16 * 16], // TODO: Remove this by using the chunk's biome!
}

impl ChunkSectionSnapshot {
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        self.blocks.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.block_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.sky_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        biome::Biome::by_id(self.biomes[((z << 4) | x) as usize] as usize)
    }
}

// TODO: make use of "x: i32", "y: i32" and "z: i32"
#[allow(dead_code)]
pub struct ChunkSectionSnapshotGroup {
    sections: [Option<ChunkSectionSnapshot>; 27],
    x: i32,
    y: i32,
    z: i32,
}

impl ChunkSectionSnapshotGroup {
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
        ChunkSectionSnapshotGroup {
            sections,
            x: -(expand_by as i32),
            y: -(expand_by as i32),
            z: -(expand_by as i32),
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        let chunk_x = ChunkSectionSnapshotGroup::cmp(x & !15, 0);
        let chunk_z = ChunkSectionSnapshotGroup::cmp(z & !15, 0);
        let chunk_y = ChunkSectionSnapshotGroup::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(block::Missing {}, |s| s.get_block(x, y, z))
    }

    pub fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let chunk_x = ChunkSectionSnapshotGroup::cmp(x & !15, 0);
        let chunk_z = ChunkSectionSnapshotGroup::cmp(z & !15, 0);
        let chunk_y = ChunkSectionSnapshotGroup::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(16, |s| s.get_block_light(x, y, z))
    }

    pub fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let chunk_x = ChunkSectionSnapshotGroup::cmp(x & !15, 0);
        let chunk_z = ChunkSectionSnapshotGroup::cmp(z & !15, 0);
        let chunk_y = ChunkSectionSnapshotGroup::cmp(y & !15, 0);
        let section = self.sections
            [((chunk_x + 1) + (chunk_z + 1) * 3 + (chunk_y + 1) * 3 * 3) as usize]
            .as_ref();
        let x = if x < 0 { 16 + x } else { x & 15 };
        let y = if y < 0 { 16 + y } else { y & 15 };
        let z = if z < 0 { 16 + z } else { z & 15 };
        section.map_or(16, |s| s.get_sky_light(x, y, z))
    }

    pub fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        let chunk_x = ChunkSectionSnapshotGroup::cmp(x & !15, 0);
        let chunk_z = ChunkSectionSnapshotGroup::cmp(z & !15, 0);
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
