use crate::render::model::ModelKey;
use crate::render::{Renderer, model};
use crate::entity::Position;
use cgmath::{Vector3, Decomposed, Rad, Quaternion, Rotation3, Matrix4};
use std::f32::consts::PI;
use crate::render;
use crate::particle::CustomParticleRenderer;
use crate::world::World;

pub struct BlockBreakEffect {
    status: i8, // 0 - 9(textures) | -1 - 10(actual values sent)(-1,0,1,...,9,10,-1)
    dirty: bool,
    position: Vector3<f64>,
    model: Option<ModelKey>,
}

impl BlockBreakEffect {
    pub(crate) fn new(manager: &mut Manager, entity: Entity) -> Self {
        let data: &BlockEffectData = manager.get_component_direct(entity).unwrap();
        manager.remove_component_direct::<BlockEffectData>(entity);
        Self {
            status: data.status,
            dirty: true,
            model: None,
            position: data.position,
        }
    }

    pub fn update(&mut self, status: i8) {
        self.status = status;
        self.dirty = true;
    }
}

pub struct BlockBreakRenderer {

    effect: Key<BlockBreakEffect>,

}

impl BlockBreakRenderer {

    pub fn new(manager: &mut Manager) -> Self {
        Self {
            effect: manager.get_key(),
        }
    }

    fn readd_model(&self, renderer: &mut Renderer, effect: &mut BlockBreakEffect) {
        if let Some(model) = effect.model.take() {
            renderer.model.remove_model(model);
        }
        if effect.status > -1 {
            let mut model = vec![];
            let tex = render::Renderer::get_texture(renderer.get_textures_ref(), &*format!("block/destroy_stage_{}", effect.status));
            model::append_box(&mut model, 0.0, 0.0, 0.0, 1.01, 1.01, 1.01, [
                Some(tex.clone()),
                Some(tex.clone()),
                Some(tex.clone()),
                Some(tex.clone()),
                Some(tex.clone()),
                Some(tex),
            ]);
            effect.model.replace(renderer.model.create_model(
                model::DEFAULT,
                vec![
                    model
                ],
            ));
        }
    }

}

impl CustomParticleRenderer for BlockBreakRenderer {
    fn update(&self, manager: &mut Manager, world: &World, renderer: &mut Renderer, entity: Entity, focused: bool, dead: bool) {
        if let Some(effect) = manager.get_component_mut(entity, self.effect) {
            if effect.dirty {
                self.readd_model(renderer, effect);
                effect.dirty = false;
            }
            if let Some(model) = effect.model {
                let mdl = renderer.model.get_model(model);
                if let Some(mdl) = mdl {
                    let offset = Vector3::new(
                        effect.position.x as f32,
                        -effect.position.y as f32,
                        effect.position.z as f32,
                    );
                    let offset_matrix = Matrix4::from(Decomposed {
                        scale: 1.0,
                        rot: Quaternion::from_angle_y(Rad(0.0)),
                        disp: offset,
                    });
                    mdl.matrix[0] = offset_matrix;
                }
            }
        }
    }

    fn particle_added(&self, manager: &mut Manager, world: &World, renderer: &mut Renderer, entity: Entity, metadata: Entity) {
        let mut effect = BlockBreakEffect::new(manager, metadata);
        self.readd_model(renderer, &mut effect);
        manager.add_component(entity, self.effect, effect);
    }

    fn particle_removed(&self, manager: &mut Manager, world: &World, renderer: &mut Renderer, entity: Entity) {
        println!("removed particle!");
        let break_effect = manager.get_component_mut(entity, self.effect).unwrap();
        if let Some(model) = break_effect.model.take() {
            renderer.model.remove_model(model);
        }
    }
}

pub struct BlockEffectData {
    pub(crate) position: Vector3<f64>,
    pub(crate) status: i8,
}