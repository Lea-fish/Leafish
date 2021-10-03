use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use leafish_blocks as block;
use leafish_protocol::types::hash::FNVHash;
use leafish_shared::position::Position;

pub use self::{chunk_section::*, chunk_snapshot::*, composed_section::*};

use crate::ecs;
use crate::world::{biome, CPos};

mod chunk_section;
mod chunk_snapshot;
mod composed_section;

pub struct Chunk {
    pub(crate) position: CPos,

    pub(crate) sections: [Option<ChunkSection>; 16],
    pub(crate) sections_rendered_on: [u32; 16],
    pub(crate) biomes: [u8; 16 * 16],

    pub(crate) heightmap: [u8; 16 * 16],
    pub(crate) heightmap_dirty: bool,

    pub(crate) block_entities: HashMap<Position, ecs::Entity, BuildHasherDefault<FNVHash>>,
}

impl Chunk {
    pub(crate) fn new(pos: CPos) -> Chunk {
        Chunk {
            position: pos,
            sections: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
            sections_rendered_on: [0; 16],
            biomes: [0; 16 * 16],
            heightmap: [0; 16 * 16],
            heightmap_dirty: true,
            block_entities: HashMap::with_hasher(BuildHasherDefault::default()),
        }
    }

    pub(crate) fn calculate_heightmap(&mut self) {
        for x in 0..16 {
            for z in 0..16 {
                let idx = ((z << 4) | x) as usize;
                for yy in 0..256 {
                    let sy = 255 - yy;
                    if let block::Air { .. } = self.get_block(x, sy, z) {
                        continue;
                    }
                    self.heightmap[idx] = sy as u8;
                    break;
                }
            }
        }
        self.heightmap_dirty = true;
    }

    pub(crate) fn set_block(&mut self, x: i32, y: i32, z: i32, b: block::Block) -> bool {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return false;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if let block::Air {} = b {
                return false;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(ChunkSection::new(s_idx as u8, fill_sky));
        }
        {
            let section = self.sections[s_idx as usize].as_mut().unwrap();
            if !section.set_block(x, y & 0xF, z, b) {
                return false;
            }
        }
        let idx = ((z << 4) | x) as usize;
        match self.heightmap[idx].cmp(&(y as u8)) {
            Ordering::Less => {
                self.heightmap[idx] = y as u8;
                self.heightmap_dirty = true;
            }
            Ordering::Equal => {
                // Find a new lowest
                for yy in 0..y {
                    let sy = y - yy - 1;
                    if let block::Air { .. } = self.get_block(x, sy, z) {
                        continue;
                    }
                    self.heightmap[idx] = sy as u8;
                    break;
                }
                self.heightmap_dirty = true;
            }
            Ordering::Greater => (),
        }
        true
    }

    pub(crate) fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return block::Missing {};
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.get_block(x, y & 0xF, z),
            None => block::Air {},
        }
    }

    pub(crate) fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return 0;
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.get_block_light(x, y & 0xF, z),
            None => 0,
        }
    }

    pub(crate) fn set_block_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if light == 0 {
                return;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(ChunkSection::new(s_idx as u8, fill_sky));
        }
        if let Some(sec) = self.sections[s_idx].as_mut() {
            sec.set_block_light(x, y & 0xF, z, light)
        }
    }

    pub(crate) fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return 15;
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.get_sky_light(x, y & 0xF, z),
            None => 15,
        }
    }

    pub(crate) fn set_sky_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if light == 15 {
                return;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(ChunkSection::new(s_idx as u8, fill_sky));
        }
        if let Some(sec) = self.sections[s_idx as usize].as_mut() {
            sec.set_sky_light(x, y & 0xF, z, light)
        }
    }

    // TODO: make use of "get_biome"
    #[allow(dead_code)]
    fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        biome::Biome::by_id(self.biomes[((z << 4) | x) as usize] as usize)
    }

    pub fn capture_snapshot(&self) -> ChunkSnapshot {
        let mut snapshot_sections = [
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None,
        ];
        for section in self.sections.iter().enumerate() {
            if section.1.is_some() {
                snapshot_sections[section.0] =
                    Some(section.1.as_ref().unwrap().capture_snapshot(self.biomes));
            }
        }
        ChunkSnapshot {
            position: self.position,
            sections: snapshot_sections,
            biomes: self.biomes,
            heightmap: self.heightmap,
        }
    }
}
