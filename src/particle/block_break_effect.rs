use crate::render::{Renderer, model};
use crate::entity::Position;
use cgmath::{Vector3, Decomposed, Rad, Quaternion, Rotation3, Matrix4};
use std::f32::consts::PI;
use crate::render;
use crate::world::World;
use crate::ecs::Manager;
use bevy_ecs::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::render::model::ModelHandle;

#[derive(Component)]
pub struct BlockBreakEffect {
    status: i8, // 0 - 9(textures) | -1 - 10(actual values sent)(-1,0,1,...,9,10,-1)
    dirty: bool,
    position: Vector3<f64>,
    model: Option<ModelHandle>,
}

impl BlockBreakEffect {
    pub(crate) fn new(data: &BlockEffectData) -> Self {
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

pub fn effect_added(renderer: Res<Arc<Renderer>>, mut commands: Commands, query: Query<(Entity, &BlockEffectData)>) {
    for (entity, data) in query.iter() {
        let mut effect = BlockBreakEffect::new(data);
        commands.entity(entity).remove::<BlockEffectData>();
        readd_model(renderer.clone(), &mut effect);
        commands.entity(entity).insert(effect);
    }
}

pub fn effect_updated(renderer: Res<Arc<Renderer>>, mut query: Query<(&mut BlockBreakEffect)>) {
    for (mut effect) in query.iter_mut() {
        if effect.dirty {
            readd_model(renderer.clone(), &mut *effect);
            effect.dirty = false;
        }
        if let Some(model) = &effect.model {
            let renderer = renderer.clone();
            let mut models = renderer.models.lock();
            let mdl = models.get_model(&model);
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

fn readd_model(renderer: Arc<Renderer>, effect: &mut BlockBreakEffect) {
    effect.model.take();
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
        effect.model.replace(renderer.clone().models.lock().create_model(
            model::DEFAULT,
            vec![
                model
            ],
            renderer
        ));
    }
}

#[derive(Component)]
pub struct BlockEffectData {
    pub(crate) position: Vector3<f64>,
    pub(crate) status: i8,
}