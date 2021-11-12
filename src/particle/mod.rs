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
    pub fn create_particle(&self, m: &mut Manager, entity: Entity) {
        if self.supported() {
            self.create_particle_internally(m, entity);
            self.create_model(m, entity);
        }
    }

    pub fn create_particle_custom_model(&self, m: &mut Manager, entity: Entity) {
        if self.supported() {
            self.create_particle_internally(m, entity);
        }
    }

    fn create_particle_internally(&self, m: &mut Manager, entity: Entity) {
        m.world.entity_mut(entity).insert(*self);
    }

    #[allow(unreachable_patterns)] // this pattern will be reachable in the future, so just ignore the warning for now
    #[allow(clippy::single_match)]
    fn create_model(&self, _m: &mut Manager, _entity: Entity) {
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
