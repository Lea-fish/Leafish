use crate::render::model::ModelKey;
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

pub struct BlockBreakEffect {
    status: i8, // 0 - 9(textures) | -1 - 10(actual values sent)(-1,0,1,...,9,10,-1)
    dirty: bool,
    position: Vector3<f64>,
    model: Option<ModelKey>,
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

pub fn effect_added(renderer: Res<Arc<RwLock<Renderer>>>, mut commands: Commands, query: Query<(Entity, &BlockEffectData)>) {
    for (entity, data) in query.iter() {
        let mut effect = BlockBreakEffect::new(data);
        commands.entity(entity).remove::<BlockEffectData>();
        readd_model(&mut *renderer.clone().write(), &mut effect);
        commands.entity(entity).insert(effect);
    }
}

pub fn effect_removed(renderer: Res<Arc<RwLock<Renderer>>>, mut query: Query<(&mut BlockBreakEffect)>) {
    for (mut break_effect) in query.iter_mut() {
        println!("removed particle!");
        if let Some(model) = break_effect.model.take() {
            renderer.clone().write().model.remove_model(&model);
        }
    }
}

pub fn effect_updated(renderer: Res<Arc<RwLock<Renderer>>>, mut query: Query<(&mut BlockBreakEffect)>) {
    for (mut effect) in query.iter_mut() {
        if effect.dirty {
            readd_model(&mut *renderer.clone().write(), &mut *effect);
            effect.dirty = false;
        }
        if let Some(model) = effect.model {
            let renderer = renderer.clone();
            let mut renderer = renderer.write();
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

fn readd_model(renderer: &mut Renderer, effect: &mut BlockBreakEffect) {
    if let Some(model) = effect.model.take() {
        renderer.model.remove_model(&model);
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

pub struct BlockEffectData {
    pub(crate) position: Vector3<f64>,
    pub(crate) status: i8,
}