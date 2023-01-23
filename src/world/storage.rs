use crate::world::block;

#[derive(Clone)]
pub struct BlockStorage {
    blocks: Vec<block::Block>,
}

impl BlockStorage {
    pub fn new(size: usize) -> Self {
        Self::new_default(size, block::Air {})
    }

    pub fn new_default(size: usize, def: block::Block) -> Self {
        Self {
            blocks: vec![def; size],
        }
    }

    pub fn get(&self, idx: usize) -> block::Block {
        self.blocks[idx]
    }

    pub fn set(&mut self, idx: usize, b: block::Block) -> bool {
        if self.blocks[idx] == b {
            false
        } else {
            self.blocks[idx] = b;
            true
        }
    }
}
