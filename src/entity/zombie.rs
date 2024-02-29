use super::{Bounds, GameInfo, Light, Position, Rotation};
use crate::entity::player_like::{compute_player_model_components, PlayerLikeModelPart};
use crate::render::model;
use crate::render::Renderer;
use bevy_ecs::prelude::*;
use cgmath::{Decomposed, Matrix4, Point3, Quaternion, Rad, Rotation3, Vector3};
use collision::Aabb3;
use std::sync::Arc;

#[derive(Component)]
pub struct ZombieModel {
    model: Option<model::ModelHandle>,
    name: Option<String>,

    dir: i32,
    time: f64,
    still_time: f64,
    idle_time: f64,
}

impl ZombieModel {
    pub fn new(name: Option<String>) -> ZombieModel {
        ZombieModel {
            model: None,
            name,
            dir: 0,
            time: 0.0,
            still_time: 0.0,
            idle_time: 0.0,
        }
    }
}

pub fn added_zombie(
    renderer: Res<Arc<Renderer>>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ZombieModel), Added<ZombieModel>>,
) {
    for (entity, mut zombie_model) in query.iter_mut() {
        commands.entity(entity).insert(Bounds::new(Aabb3::new(
            Point3::new(-0.3, 0.0, -0.3),
            Point3::new(0.3, 1.8, 0.3),
        )));
        let tex = Renderer::get_texture(
            renderer.get_textures_ref(),
            "minecraft:entity/zombie/zombie",
        );
        let components =
            compute_player_model_components(&tex, &zombie_model.name, renderer.clone());

        zombie_model
            .model
            .replace(renderer.clone().models.lock().create_model(
                model::DEFAULT,
                components,
                renderer.clone(),
            ));
    }
}

pub fn update_zombie(
    game_info: Res<GameInfo>,
    renderer: Res<Arc<Renderer>>,
    mut query: Query<(&mut ZombieModel, &Position, &Rotation, &Light)>,
) {
    for (mut zombie_model, position, rotation, light) in query.iter_mut() {
        use std::f32::consts::PI;
        use std::f64::consts::PI as PI64;
        let delta = game_info.delta;

        if let Some(zmodel) = &zombie_model.model {
            let renderer = renderer.clone();
            let mut models = renderer.models.lock();
            let mdl = models.get_model(zmodel).unwrap();

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

            // TODO This sucks
            /* if zombie_model.has_name_tag {
                let ang = (position.position.x - renderer.camera.pos.x)
                    .atan2(position.position.z - renderer.camera.pos.z)
                    as f32;
                mdl.matrix[ZombieModelPart::NameTag as usize] = Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_y(Rad(ang)),
                    disp: offset + Vector3::new(0.0, (-24.0 / 16.0) - 0.6, 0.0),
                });
            }*/

            mdl.matrix[PlayerLikeModelPart::Head as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(-rotation.pitch as f32)),
                    disp: Vector3::new(0.0, -12.0 / 16.0 - 12.0 / 16.0, 0.0),
                });
            mdl.matrix[PlayerLikeModelPart::Body as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(0.0)),
                    disp: Vector3::new(0.0, -12.0 / 16.0 - 6.0 / 16.0, 0.0),
                });

            let mut time = zombie_model.time;
            let mut dir = zombie_model.dir;
            if dir == 0 {
                dir = 1;
                time = 15.0;
            }
            let ang = ((time / 15.0) - 1.0) * (PI64 / 4.0);

            mdl.matrix[PlayerLikeModelPart::LegRight as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(ang as f32)),
                    disp: Vector3::new(2.0 / 16.0, -12.0 / 16.0, 0.0),
                });
            mdl.matrix[PlayerLikeModelPart::LegLeft as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(-ang as f32)),
                    disp: Vector3::new(-2.0 / 16.0, -12.0 / 16.0, 0.0),
                });

            let mut i_time = zombie_model.idle_time;
            i_time += delta * 0.02;
            if i_time > PI64 * 2.0 {
                i_time -= PI64 * 2.0;
            }
            zombie_model.idle_time = i_time;

            mdl.matrix[PlayerLikeModelPart::ArmRight as usize] = offset_matrix
                * Matrix4::from_translation(Vector3::new(
                    6.0 / 16.0,
                    -12.0 / 16.0 - 12.0 / 16.0,
                    0.0,
                ))
                * Matrix4::from(Quaternion::from_angle_x(Rad(-(ang * 0.75) as f32)))
                * Matrix4::from(Quaternion::from_angle_z(Rad(
                    (i_time.cos() * 0.06 - 0.06) as f32
                )))
                * Matrix4::from(Quaternion::from_angle_x(Rad((i_time.sin() * 0.06
                    - ((7.5 - (0.5_f64 - 7.5).abs()) / 7.5))
                    as f32)));

            mdl.matrix[PlayerLikeModelPart::ArmLeft as usize] = offset_matrix
                * Matrix4::from_translation(Vector3::new(
                    -6.0 / 16.0,
                    -12.0 / 16.0 - 12.0 / 16.0,
                    0.0,
                ))
                * Matrix4::from(Quaternion::from_angle_x(Rad((ang * 0.75) as f32)))
                * Matrix4::from(Quaternion::from_angle_z(Rad(
                    -(i_time.cos() * 0.06 - 0.06) as f32
                )))
                * Matrix4::from(Quaternion::from_angle_x(Rad(-(i_time.sin() * 0.06) as f32)));

            let mut update = true;
            if position.moved {
                zombie_model.still_time = 0.0;
            } else if zombie_model.still_time > 2.0 {
                if (time - 15.0).abs() <= 1.5 * delta {
                    time = 15.0;
                    update = false;
                }
                dir = (15.0 - time).signum() as i32;
            } else {
                zombie_model.still_time += delta;
            }

            if update {
                time += delta * 1.5 * (dir as f64);
                if time > 30.0 {
                    time = 30.0;
                    dir = -1;
                } else if time < 0.0 {
                    time = 0.0;
                    dir = 1;
                }
            }
            zombie_model.time = time;
            zombie_model.dir = dir;
        }
    }
}
