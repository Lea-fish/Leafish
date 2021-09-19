use super::{
    Bounds, GameInfo, Gravity, Light, Position, Rotation, TargetPosition, TargetRotation, Velocity,
};
use crate::ecs;
use crate::format;
use crate::render;
use crate::render::model::{self, FormatState};
use crate::settings::Actionkey;
use crate::shared::Position as BPosition;
use crate::types::hash::FNVHash;
use crate::types::GameMode;
use crate::world;
use cgmath::{self, Decomposed, Matrix4, Point3, Quaternion, Rad, Rotation3, Vector3};
use collision::{Aabb, Aabb3};
use instant::Instant;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use crate::entity::{CustomEntityRenderer, EntityType, resolve_textures};
use crate::render::{Renderer, Texture};
use crate::ecs::Entity;
use crate::entity::player_like::{compute_player_model_components, PlayerLikeModelPart};

pub struct ZombieModel {
    model: Option<model::ModelKey>,
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

pub fn create_zombie(m: &mut ecs::Manager) -> ecs::Entity {
    let entity = m.create_entity();
    m.add_component_direct(entity, Position::new(1478.5, 47.0, -474.5));
    m.add_component_direct(entity, Rotation::new(0.0, 0.0));
    m.add_component_direct(entity, Velocity::new(0.0, 0.0, 0.0));
    m.add_component_direct(
        entity,
        Bounds::new(Aabb3::new(
            Point3::new(-0.3, 0.0, -0.3),
            Point3::new(0.3, 1.8, 0.3),
        )),
    );
    m.add_component_direct(entity, Light::new());
    m.add_component_direct(entity, EntityType::Zombie);
    m.add_component_direct(entity, ZombieModel::new(Some(String::from("test"))));
    entity
}

pub struct ZombieRenderer {
    zombie_model: ecs::Key<ZombieModel>,
    position: ecs::Key<Position>,
    rotation: ecs::Key<Rotation>,
    game_info: ecs::Key<GameInfo>,
    light: ecs::Key<Light>,
}

impl ZombieRenderer {
    pub fn new(m: &mut ecs::Manager) -> Self {
        let zombie_model = m.get_key();
        let position = m.get_key();
        let rotation = m.get_key();
        let light = m.get_key();
        ZombieRenderer {
            zombie_model,
            position,
            rotation,
            game_info: m.get_key(),
            light,
        }
    }
}

// TODO: Setup culling
impl CustomEntityRenderer for ZombieRenderer {
    fn update(
        &self,
        m: &mut ecs::Manager,
        _world: &world::World,
        renderer: &mut render::Renderer,
        _: bool,
        _: bool,
        e: Entity
    ) {
        use std::f32::consts::PI;
        use std::f64::consts::PI as PI64;
        let world_entity = m.get_world();
        let delta = m
            .get_component_mut(world_entity, self.game_info)
            .unwrap()
            .delta;
        let player_model = m.get_component_mut(e, self.zombie_model).unwrap();
        let position = m.get_component_mut(e, self.position).unwrap();
        let rotation = m.get_component_mut(e, self.rotation).unwrap();
        let light = m.get_component(e, self.light).unwrap();

        if let Some(pmodel) = player_model.model {
            let mdl = renderer.model.get_model(pmodel).unwrap();

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
           /* if player_model.has_name_tag {
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

            let mut time = player_model.time;
            let mut dir = player_model.dir;
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

            let mut i_time = player_model.idle_time;
            i_time += delta * 0.02;
            if i_time > PI64 * 2.0 {
                i_time -= PI64 * 2.0;
            }
            player_model.idle_time = i_time;

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
                player_model.still_time = 0.0;
            } else if player_model.still_time > 2.0 {
                if (time - 15.0).abs() <= 1.5 * delta {
                    time = 15.0;
                    update = false;
                }
                dir = (15.0 - time).signum() as i32;
            } else {
                player_model.still_time += delta;
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
            player_model.time = time;
            player_model.dir = dir;
        }
    }

    fn entity_added(
        &self,
        m: &mut ecs::Manager,
        e: ecs::Entity,
        _: &world::World,
        renderer: &mut render::Renderer,
    ) {
        let zombie_model = m.get_component_mut(e, self.zombie_model).unwrap();
        let tex = Renderer::get_texture(renderer.get_textures_ref(), "minecraft:entity/zombie/zombie");
        let components = compute_player_model_components(&tex, &zombie_model.name, renderer);

        zombie_model.model = Some(renderer.model.create_model(
            model::DEFAULT,
            components,
        ));
    }

    fn entity_removed(
        &self,
        m: &mut ecs::Manager,
        e: ecs::Entity,
        _: &world::World,
        renderer: &mut render::Renderer,
    ) {
        let zombie_model = m.get_component_mut(e, self.zombie_model).unwrap();
        if let Some(model) = zombie_model.model.take() {
            renderer.model.remove_model(model);
        }
    }
}