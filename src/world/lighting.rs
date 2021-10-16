use std::io::Cursor;

use leafish_shared::position::Position;

use crate::world::World;

pub struct LightData {
    pub arrays: Cursor<Vec<u8>>,
    pub block_light_mask: i32,
    pub sky_light_mask: i32,
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
