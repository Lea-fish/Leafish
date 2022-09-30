use bevy_ecs::prelude::*;

pub mod block_break_effect;

#[derive(Component, Copy, Clone)]
pub struct EntityMetadata(pub Entity);

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParticleType {
    BlockBreak,
}
