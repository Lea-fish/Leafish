use leafish_blocks as block;
use std::cmp::Ordering;
use std::sync::Arc;

use crate::world::biome::Biome;
use crate::world::chunk::chunk_section::ChunkSectionSnapshot;
use crate::world::{biome, CPos, World, EMPTY_SECTION};

// TODO: make use of "x: i32", "y: i32" and "z: i32"
#[allow(dead_code)]
pub struct ComposedSection {
    sections: [Option<ChunkSectionSnapshot>; 27],
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
