use crate::world::chunk::chunk_section::SectionSnapshot;
use crate::world::CPos;

pub struct ChunkSnapshot {
    pub position: CPos,
    pub sections: [Option<SectionSnapshot>; 16],
    pub biomes: [u8; 16 * 16],
    pub heightmap: [u8; 16 * 16],
}
