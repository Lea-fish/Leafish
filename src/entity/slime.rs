use super::{Bounds, GameInfo, Light, Position, Rotation, Velocity};
use crate::ecs::Manager;
use crate::entity::{resolve_textures, EntityType};
use crate::render;
use crate::render::model;
use crate::render::Renderer;
use crate::world;
use cgmath::{self, Decomposed, Matrix4, Point3, Quaternion, Rad, Rotation3, Vector3};
use collision::Aabb3;
use bevy_ecs::prelude::*;
use std::sync::Arc;
use parking_lot::{RwLock, Mutex};

#[derive(Component)]
pub struct SlimeModel {
    model: Option<model::ModelHandle>,
    _name: String,

    _dir: i32,
    _time: f64,
    _still_time: f64,
    idle_time: f64,
}

impl SlimeModel {
    pub fn new(name: &str) -> SlimeModel {
        SlimeModel {
            model: None,
            _name: name.to_owned(),

            _dir: 0,
            _time: 0.0,
            _still_time: 0.0,
            idle_time: 0.0,
        }
    }
}

pub fn create_slime(m: &mut Manager) -> Entity {
    let mut entity = m.world.spawn();
    entity.insert(Position::new(1478.5, 44.0, -474.5))
        .insert(Rotation::new(0.0, 0.0))
        .insert(Velocity::new(0.0, 0.0, 0.0))
        .insert(Bounds::new(Aabb3::new(
            Point3::new(-0.3, 0.0, -0.3),
            Point3::new(0.3, 1.8, 0.3),
        )))
        .insert(Light::new())
        .insert(EntityType::Slime)
        .insert(SlimeModel::new("test"));
    entity.id()
}



pub fn update_slime(game_info: Res<GameInfo>, renderer: Res<Arc<Renderer>>, mut query: Query<(&mut SlimeModel, &Position, &Rotation, &Light)>) {
   for (mut slime_model, position, rotation, light) in query.iter_mut() {
       use std::f32::consts::PI;
       use std::f64::consts::PI as PI64;
       let delta = game_info
           .delta;

       /*if slime_model.dirty {
           self.entity_removed(m, e, world, renderer);
           self.entity_added(m, e, world, renderer);
       }*/

       if let Some(pmodel) = &slime_model.model.clone() {
           let renderer = renderer.clone();
           let mut models = renderer.models.lock();
           let mdl = models.get_model(&pmodel).unwrap();

           mdl.block_light = light.block_light;
           mdl.sky_light = light.sky_light;

           let offset = Vector3::new(
               position.position.x as f32,
               -position.position.y as f32,
               position.position.z as f32,
           );
           let offset_matrix = Matrix4::from(Decomposed {
               scale: 1.0,
               rot: Quaternion::from_angle_y(Rad(PI + rotation.yaw as f32)),
               disp: offset,
           });

           mdl.matrix[SlimeModelPart::Body as usize] = offset_matrix
               * Matrix4::from(Decomposed {
               scale: 1.0,
               rot: Quaternion::from_angle_x(Rad(0.0)),
               disp: Vector3::new(0.0, -12.0 / 16.0 - 6.0 / 16.0, 0.0),
           });

           mdl.matrix[SlimeModelPart::Eyes as usize] = offset_matrix
               * Matrix4::from(Decomposed {
               scale: 1.0,
               rot: Quaternion::from_angle_x(Rad(0.0)),
               disp: Vector3::new(0.0, -12.0 / 16.0 - 6.0 / 16.0, 0.0),
           });

           // TODO This sucks
           /*if slime_model.has_name_tag {
               let ang = (position.position.x - renderer.camera.pos.x)
                   .atan2(position.position.z - renderer.camera.pos.z)
                   as f32;
               mdl.matrix[SlimeModelPart::NameTag as usize] = Matrix4::from(Decomposed {
                   scale: 1.0,
                   rot: Quaternion::from_angle_y(Rad(ang)),
                   disp: offset + Vector3::new(0.0, (-24.0 / 16.0) - 0.6, 0.0),
               });
           }*/

           let mut i_time = slime_model.idle_time;
           i_time += delta * 0.02;
           if i_time > PI64 * 2.0 {
               i_time -= PI64 * 2.0;
           }
           slime_model.idle_time = i_time;
       }
   }
}

pub fn added_slime(renderer: Res<Arc<Renderer>>, mut query: Query<(&mut SlimeModel)>) {
    for (mut slime_model) in query.iter_mut() {
        let tex =
            Renderer::get_texture(renderer.get_textures_ref(), "minecraft:entity/slime/slime");
        let mut body_verts = vec![];
        model::append_box(
            &mut body_verts,
            -4.0 / 16.0,
            16.0 / 16.0,
            -4.0 / 16.0,
            8.0 / 16.0,
            8.0 / 16.0,
            8.0 / 16.0,
            resolve_textures(&tex, 8.0, 8.0, 8.0, 0.0, 0.0),
        );
        model::append_box(
            &mut body_verts,
            -3.0 / 16.0,
            17.0 / 16.0,
            -3.0 / 16.0,
            6.0 / 16.0,
            6.0 / 16.0,
            6.0 / 16.0,
            resolve_textures(&tex, 6.0, 6.0, 6.0, 0.0, 0.0),
        );

        let mut eye_verts = vec![];

        model::append_box(
            // right eye
            &mut eye_verts,
            -3.25 / 16.0,
            18.0 / 16.0,
            -3.5 / 16.0,
            2.0 / 16.0,
            2.0 / 16.0,
            2.0 / 16.0,
            resolve_textures(&tex, 2.0, 2.0, 2.0, 32.0, 0.0),
        );
        model::append_box(
            // left eye
            &mut eye_verts,
            1.25 / 16.0,
            18.0 / 16.0,
            -3.5 / 16.0,
            2.0 / 16.0,
            2.0 / 16.0,
            2.0 / 16.0,
            resolve_textures(&tex, 2.0, 2.0, 2.0, 32.0, 4.0),
        );

        // let mut name_verts = vec![];
        /*if slime_model.has_name_tag {
            let mut state = FormatState {
                width: 0.0,
                offset: 0.0,
                text: Vec::new(),
                renderer,
                y_scale: 0.16,
                x_scale: 0.01,
            };
            let mut name = format::Component::Text(format::TextComponent::new(&slime_model.name));
            format::convert_legacy(&mut name);
            state.build(&name, format::Color::Black);
            // TODO: Remove black shadow and add dark, transparent box around name
            let width = state.width;
            // Center align text
            for vert in &mut state.text {
                vert.x += width * 0.5;
                vert.r = 64;
                vert.g = 64;
                vert.b = 64;
            }
            name_verts.extend_from_slice(&state.text);
            for vert in &mut state.text {
                vert.x -= 0.01;
                vert.y -= 0.01;
                vert.z -= 0.05;
                vert.r = 255;
                vert.g = 255;
                vert.b = 255;
            }
            name_verts.extend_from_slice(&state.text);
        }*/
        let model = renderer.clone().models.lock().create_model(
            model::DEFAULT,
            vec![
                body_verts, eye_verts,
                // name_verts,
            ],
            renderer.clone(),
        );

        slime_model.model.replace(model);
    }
}

enum SlimeModelPart {
    Body = 0,
    Eyes = 1,
}
