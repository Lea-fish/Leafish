use crate::world::{biome, storage};
use crate::{chunk_builder, render};
pub use leafish_blocks as block;
use leafish_protocol::types::nibble;
use parking_lot::RwLock;
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
