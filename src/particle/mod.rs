use std::sync::Arc;
use crate::world::World;
use crate::render::Renderer;
use dashmap::DashMap;
use lazy_static::lazy_static;
use crate::entity::{Velocity, Rotation, Position, TargetPosition, TargetRotation};
use crate::particle::block_break_effect::{BlockBreakEffect};
use crate::ecs::Manager;
use bevy_ecs::prelude::*;

pub mod block_break_effect;

#[derive(Component, Copy, Clone)]
pub struct EntityMetadata(pub Entity);

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParticleType {

    BlockBreak,

}

impl ParticleType {

    pub fn create_particle(
        &self,
        m: &mut Manager,
        entity: Entity,
    ) {
        if self.supported() {
            self.create_particle_internally(m, entity);
            self.create_model(m, entity);
        }
    }

    pub fn create_particle_custom_model(
        &self,
        m: &mut Manager,
        entity: Entity,
    ) {
        if self.supported() {
            self.create_particle_internally(m, entity);
        }
    }

    fn create_particle_internally(
        &self,
        m: &mut Manager,
        entity: Entity,
    ) {
        m.world.entity_mut(entity).insert(*self);
    }

    fn create_model(&self, m: &mut Manager, entity: Entity) {
        match self {
            ParticleType::BlockBreak => {
                /*let effect = BlockBreakEffect::new(m, metadata);
                m.add_component_direct(entity, effect);*/
            }
            _ => {}
        };
    }

    fn supported(&self) -> bool {
        matches!(self, ParticleType::BlockBreak)
    }
}