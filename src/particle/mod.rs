use std::sync::Arc;
use crate::world::World;
use crate::render::Renderer;
use dashmap::DashMap;
use lazy_static::lazy_static;
use crate::entity::{Velocity, Rotation, Position, TargetPosition, TargetRotation};
use crate::particle::block_break_effect::{BlockBreakEffect, BlockBreakRenderer};

pub mod block_break_effect;

pub struct ParticleRenderer {

    filter: Filter,
    particle_type: Key<ParticleType>,
    metadata: Key<Entity>,

}

impl ParticleRenderer {
    pub fn new(manager: &mut Manager) -> Self {
        let particle_type = manager.get_key();
        let metadata = manager.get_key();
        Self {
            filter: Filter::new()
                .with(particle_type)
                .with(metadata),
            particle_type,
            metadata,
        }
    }
}

impl System for ParticleRenderer {
    fn filter(&self) -> &Filter {
        &self.filter
    }

    fn update(
        &mut self,
        m: &mut Manager,
        world: &World,
        renderer: &mut Renderer,
        focused: bool,
        dead: bool,
    ) {
        for e in m.find(&self.filter) {
            let particle_type = m.get_component(e, self.particle_type).unwrap();
            let c_renderer = particle_type.get_renderer();
            c_renderer.update(m, world, renderer, e, focused, dead);
        }
    }

    fn entity_added(&mut self, m: &mut Manager, world: &World, renderer: &mut Renderer, e: Entity) {
        let particle_type = m.get_component(e, self.particle_type).unwrap();
        let c_renderer = particle_type.get_renderer();
        let metadata = m.get_component(e, self.metadata).unwrap();
        c_renderer.particle_added(m, world, renderer, e, *metadata);
    }

    fn entity_removed(
        &mut self,
        m: &mut Manager,
        world: &World,
        renderer: &mut Renderer,
        e: Entity,
    ) {
        let particle_type = m.get_component(e, self.particle_type).unwrap();
        let c_renderer = particle_type.get_renderer();
        c_renderer.particle_removed(m, world, renderer, e);
    }
}

pub trait CustomParticleRenderer {

    fn update(
        &self,
        manager: &mut Manager,
        world: &World,
        renderer: &mut Renderer,
        entity: Entity,
        focused: bool,
        dead: bool,
    );

    fn particle_added(
        &self,
        manager: &mut Manager,
        world: &World,
        renderer: &mut Renderer,
        entity: Entity,
        metadata: Entity,
    );

    fn particle_removed(
        &self,
        manager: &mut Manager,
        world: &World,
        renderer: &mut Renderer,
        entity: Entity,
    );

}

pub struct NOOPParticleRenderer {}

impl CustomParticleRenderer for NOOPParticleRenderer {
    fn update(&self, _: &mut Manager, _: &World, _: &mut Renderer, _: Entity, _: bool, _: bool) {}

    fn particle_added(&self, _: &mut Manager, _: &World, _: &mut Renderer, _: Entity, _: Entity) {}

    fn particle_removed(&self, _: &mut Manager, _: &World, _: &mut Renderer, _: Entity) {}
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParticleType {

    BlockBreak,

}

lazy_static! {
    static ref PARTICLE_RENDERERS: Arc<DashMap<ParticleType, Arc<dyn CustomParticleRenderer + Send + Sync>>> =
        Arc::new(DashMap::new());
    static ref NOOP_RENDERER: Arc<dyn CustomParticleRenderer + Send + Sync> =
        Arc::new(NOOPParticleRenderer {});
}

impl ParticleType {

    pub fn init(manager: &mut Manager) {
        PARTICLE_RENDERERS.insert(ParticleType::BlockBreak, Arc::new(BlockBreakRenderer::new(manager)));
    }

    pub fn deinit() {
        PARTICLE_RENDERERS.clear();
    }

    pub fn get_renderer(&self) -> Arc<dyn CustomParticleRenderer + Send + Sync> {
        PARTICLE_RENDERERS
            .get(self)
            .map_or(NOOP_RENDERER.clone(), |x| x.value().clone())
    }

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
        let entity = m.create_entity();
        m.add_component_direct(entity, metadata);
        m.add_component_direct(entity, *self);
        entity
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