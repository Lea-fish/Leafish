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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParticleType {

    BlockBreak,

}

impl ParticleType {

    pub fn create_particle(
        &self,
        m: &mut Manager,
        metadata: Entity,
    ) -> Option<Entity> {
        if self.supported() {
            let ret = self.create_particle_internally(m, metadata);
            self.create_model(m, ret, metadata);
            return Some(ret);
        }
        None
    }

    pub fn create_particle_custom_model(
        &self,
        m: &mut Manager,
        metadata: Entity,
    ) -> Option<Entity> {
        if self.supported() {
            return Some(self.create_particle_internally(m, metadata));
        }
        None
    }

    fn create_particle_internally(
        &self,
        m: &mut Manager,
        metadata: Entity
    ) -> Entity {
        let mut entity = m.world.spawn();
        entity.insert(metadata)
            .insert(*self);
        entity.id()
    }

    fn create_model(&self, m: &mut Manager, entity: Entity, metadata: Entity) {
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