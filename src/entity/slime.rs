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
use crate::entity::{CustomEntityRenderer, EntityType};
use crate::render::{Renderer, Texture};
use crate::ecs::Entity;

static TEXTURE_MATRIX: [[f32; 2]; 6] = [
    [2.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [3.0, 1.0],
    [2.0, 1.0],
    [0.0, 1.0],
];

pub struct SlimeModel {
    model: Option<model::ModelKey>,
    name: String,

    dir: i32,
    time: f64,
    still_time: f64,
    idle_time: f64,
}

impl SlimeModel {
    pub fn new(name: &str) -> SlimeModel {
        SlimeModel {
            model: None,
            name: name.to_owned(),

            dir: 0,
            time: 0.0,
            still_time: 0.0,
            idle_time: 0.0,
        }
    }
}

pub fn create_slime(m: &mut ecs::Manager) -> ecs::Entity {
    let entity = m.create_entity();
    m.add_component_direct(entity, Position::new(1478.5, 44.0, -474.5));
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
    m.add_component_direct(entity, EntityType::Slime);
    m.add_component_direct(entity, SlimeModel::new("test"));
    entity
}

pub struct SlimeRenderer {
    slime_model: ecs::Key<SlimeModel>,
    position: ecs::Key<Position>,
    rotation: ecs::Key<Rotation>,
    game_info: ecs::Key<GameInfo>,
    light: ecs::Key<Light>,
}

impl SlimeRenderer {
    pub fn new(m: &mut ecs::Manager) -> Self {
        let slime_model = m.get_key();
        let position = m.get_key();
        let rotation = m.get_key();
        let light = m.get_key();
        SlimeRenderer {
            slime_model,
            position,
            rotation,
            game_info: m.get_key(),
            light,
        }
    }
}

enum SlimeModelPart {
    Head = 0,
    Body = 1,
    LegLeft = 2,
    LegRight = 3,
    ArmLeft = 4,
    ArmRight = 5,
    NameTag = 6,
    // Cape = 7, // TODO
}

// TODO: Setup culling
impl CustomEntityRenderer for SlimeRenderer {
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
        println!("render slime!");
        let world_entity = m.get_world();
        let delta = m
            .get_component_mut(world_entity, self.game_info)
            .unwrap()
            .delta;
            let slime_model = m.get_component_mut(e, self.slime_model).unwrap();
            let position = m.get_component_mut(e, self.position).unwrap();
            let rotation = m.get_component_mut(e, self.rotation).unwrap();
            let light = m.get_component(e, self.light).unwrap();

            /*if slime_model.dirty {
                self.entity_removed(m, e, world, renderer);
                self.entity_added(m, e, world, renderer);
            }*/
        /*self.entity_removed(m, e, _world, renderer);
        self.entity_added(m, e, _world, renderer);*/

            if let Some(pmodel) = slime_model.model {
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

                mdl.matrix[0/*SlimeModelPart::Body as usize*/] = offset_matrix // TODO readd
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

               /* mdl.matrix[SlimeModelPart::Head as usize] = offset_matrix
                    * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(-rotation.pitch as f32)),
                    disp: Vector3::new(0.0, -12.0 / 16.0 - 12.0 / 16.0, 0.0),
                });
                mdl.matrix[SlimeModelPart::Body as usize] = offset_matrix
                    * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(0.0)),
                    disp: Vector3::new(0.0, -12.0 / 16.0 - 6.0 / 16.0, 0.0),
                });

                let mut time = slime_model.time;
                let mut dir = slime_model.dir;
                if dir == 0 {
                    dir = 1;
                    time = 15.0;
                }
                let ang = ((time / 15.0) - 1.0) * (PI64 / 4.0);

                mdl.matrix[SlimeModelPart::LegRight as usize] = offset_matrix
                    * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(ang as f32)),
                    disp: Vector3::new(2.0 / 16.0, -12.0 / 16.0, 0.0),
                });
                mdl.matrix[SlimeModelPart::LegLeft as usize] = offset_matrix
                    * Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_x(Rad(-ang as f32)),
                    disp: Vector3::new(-2.0 / 16.0, -12.0 / 16.0, 0.0),
                });*/

                let mut i_time = slime_model.idle_time;
                i_time += delta * 0.02;
                if i_time > PI64 * 2.0 {
                    i_time -= PI64 * 2.0;
                }
                slime_model.idle_time = i_time;

                /*
                mdl.matrix[SlimeModelPart::ArmRight as usize] = offset_matrix
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
                    - (1.0 / 7.5))
                    as f32)));

                mdl.matrix[SlimeModelPart::ArmLeft as usize] = offset_matrix
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

                */
                /*
                let mut update = true;
                if position.moved {
                    slime_model.still_time = 0.0;
                } else if slime_model.still_time > 2.0 {
                    if (time - 15.0).abs() <= 1.5 * delta {
                        time = 15.0;
                        update = false;
                    }
                    dir = (15.0 - time).signum() as i32;
                } else {
                    slime_model.still_time += delta;
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
                slime_model.time = time;
                slime_model.dir = dir;*/
            }
    }

    fn entity_added(
        &self,
        m: &mut ecs::Manager,
        e: ecs::Entity,
        _: &world::World,
        renderer: &mut render::Renderer,
    ) {
        let slime_model = m.get_component_mut(e, self.slime_model).unwrap();
        let tex = Renderer::get_texture(renderer.get_textures_ref(), "minecraft:entity/slime/slime");

        macro_rules! srel {
            ($x:expr, $y:expr, $w:expr, $h:expr) => {
                Some(tex.relative(($x) / 64.0, ($y) / 32.0, ($w) / 64.0, ($h) / 32.0))
            };
        }


        /*
        if player_model.has_head {
            model::append_box(
                &mut head_verts,
                -4.0 / 16.0,
                0.0,
                -4.0 / 16.0,
                8.0 / 16.0,
                8.0 / 16.0,
                8.0 / 16.0,
                [
                    srel!(16.0, 0.0, 8.0, 8.0), // Down
                    srel!(8.0, 0.0, 8.0, 8.0),  // Up
                    srel!(8.0, 8.0, 8.0, 8.0),  // North
                    srel!(24.0, 8.0, 8.0, 8.0), // South
                    srel!(16.0, 8.0, 8.0, 8.0), // West
                    srel!(0.0, 8.0, 8.0, 8.0),  // East
                ],
            );
            model::append_box(
                &mut head_verts,
                -4.2 / 16.0,
                -0.2 / 16.0,
                -4.2 / 16.0,
                8.4 / 16.0,
                8.4 / 16.0,
                8.4 / 16.0,
                [
                    srel!((16.0 + 32.0), 0.0, 8.0, 8.0), // Down
                    srel!((8.0 + 32.0), 0.0, 8.0, 8.0),  // Up
                    srel!((8.0 + 32.0), 8.0, 8.0, 8.0),  // North
                    srel!((24.0 + 32.0), 8.0, 8.0, 8.0), // South
                    srel!((16.0 + 32.0), 8.0, 8.0, 8.0), // West
                    srel!((0.0 + 32.0), 8.0, 8.0, 8.0),  // East
                ],
            );
        }
        */

        // TODO: Cape
        let mut body_verts = vec![];
        model::append_box(
            &mut body_verts,
            -4.0 / 8.0,
            -4.0 / 8.0,
            -4.0 / 8.0,
            8.0 / 8.0,
            8.0 / 8.0,
            8.0 / 8.0,
            resolve_textures(&tex, 8.0, 8.0, 0.0, 0.0)
            /*[
                /*srel!(8.0, 8.0, 8.0, 8.0),  // Down
                srel!(16.0, 8.0, 8.0, 8.0),  // Up
                srel!(0.0, 16.0, 8.0, 8.0), // North
                srel!(0.0, 8.0, 8.0, 8.0), // South
                srel!(0.0, 24.0, 8.0, 8.0), // West
                srel!(0.0, 24.0, 8.0, 8.0),*/ // East
                srel!(16.0, 0.0, 8.0, 8.0),  // Down
                srel!(8.0, 0.0, 8.0, 8.0),  // Up
                srel!(8.0, 8.0, 8.0, 8.0), // North
                srel!(24.0, 8.0, 8.0, 8.0), // South
                srel!(16.0, 8.0, 8.0, 8.0), // West
                srel!(0.0, 8.0, 8.0, 8.0), // East
            ],*/
        );
        /*model::append_box(
            &mut body_verts,
            -4.0 / 16.0,
            -6.0 / 16.0,
            -2.0 / 16.0,
            8.0 / 16.0,
            12.0 / 16.0,
            4.0 / 16.0,
            [
                srel!(28.0, 16.0, 8.0, 4.0),  // Down
                srel!(20.0, 16.0, 8.0, 4.0),  // Up
                srel!(20.0, 20.0, 8.0, 12.0), // North
                srel!(32.0, 20.0, 8.0, 12.0), // South
                srel!(16.0, 20.0, 4.0, 12.0), // West
                srel!(28.0, 20.0, 4.0, 12.0), // East
            ],
        );*/
        /*model::append_box(
            &mut body_verts,
            -4.2 / 16.0,
            -6.2 / 16.0,
            -2.2 / 16.0,
            8.4 / 16.0,
            12.4 / 16.0,
            4.4 / 16.0,
            [
                srel!(28.0, 16.0 + 16.0, 8.0, 4.0),  // Down
                srel!(20.0, 16.0 + 16.0, 8.0, 4.0),  // Up
                srel!(20.0, 20.0 + 16.0, 8.0, 12.0), // North
                srel!(32.0, 20.0 + 16.0, 8.0, 12.0), // South
                srel!(16.0, 20.0 + 16.0, 4.0, 12.0), // West
                srel!(28.0, 20.0 + 16.0, 4.0, 12.0), // East
            ],
        );*/

        /*
        for (i, offsets) in [
            [16.0, 48.0, 0.0, 48.0],  // Left leg
            [0.0, 16.0, 0.0, 32.0],   // Right Leg
            [32.0, 48.0, 48.0, 48.0], // Left arm
            [40.0, 16.0, 40.0, 32.0], // Right arm
        ]
            .iter()
            .enumerate()
        {
            let (ox, oy) = (offsets[0], offsets[1]);
            model::append_box(
                &mut part_verts[i],
                -2.0 / 16.0,
                -12.0 / 16.0,
                -2.0 / 16.0,
                4.0 / 16.0,
                12.0 / 16.0,
                4.0 / 16.0,
                [
                    srel!(ox + 8.0, oy + 0.0, 4.0, 4.0),     // Down
                    srel!(ox + 4.0, oy + 0.0, 4.0, 4.0),     // Up
                    srel!(ox + 4.0, oy + 4.0, 4.0, 12.0),  // North
                    srel!(ox + 12.0, oy + 4.0, 4.0, 12.0), // South
                    srel!(ox + 8.0, oy + 4.0, 4.0, 12.0),  // West
                    srel!(ox + 0.0, oy + 4.0, 4.0, 12.0),  // East
                ],
            );
            let (ox, oy) = (offsets[2], offsets[3]);
            model::append_box(
                &mut part_verts[i],
                -2.2 / 16.0,
                -12.2 / 16.0,
                -2.2 / 16.0,
                4.4 / 16.0,
                12.4 / 16.0,
                4.4 / 16.0,
                [
                    srel!(ox + 8.0, oy + 0.0, 4.0, 4.0),   // Down
                    srel!(ox + 4.0, oy + 0.0, 4.0, 4.0),   // Up
                    srel!(ox + 4.0, oy + 4.0, 4.0, 12.0),  // North
                    srel!(ox + 12.0, oy + 4.0, 4.0, 12.0), // South
                    srel!(ox + 8.0, oy + 4.0, 4.0, 12.0),  // West
                    srel!(ox + 0.0, oy + 4.0, 4.0, 12.0),  // East
                ],
            );
        }*/

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

        slime_model.model = Some(renderer.model.create_model(
            model::DEFAULT,
            vec![
                body_verts,
                // name_verts,
            ],
        ));
    }

    fn entity_removed(
        &self,
        m: &mut ecs::Manager,
        e: ecs::Entity,
        _: &world::World,
        renderer: &mut render::Renderer,
    ) {
        let slime_model = m.get_component_mut(e, self.slime_model).unwrap();
        if let Some(model) = slime_model.model.take() {
            renderer.model.remove_model(model);
        }
    }
}

trait Collidable<T> {
    fn collides(&self, t: &T) -> bool;
    fn move_out_of(self, other: Self, dir: cgmath::Vector3<f64>) -> Self;
}

impl Collidable<Aabb3<f64>> for Aabb3<f64> {
    fn collides(&self, t: &Aabb3<f64>) -> bool {
        !(t.min.x >= self.max.x
            || t.max.x <= self.min.x
            || t.min.y >= self.max.y
            || t.max.y <= self.min.y
            || t.min.z >= self.max.z
            || t.max.z <= self.min.z)
    }

    fn move_out_of(mut self, other: Self, dir: cgmath::Vector3<f64>) -> Self {
        if dir.x != 0.0 {
            if dir.x > 0.0 {
                let ox = self.max.x;
                self.max.x = other.min.x - 0.0001;
                self.min.x += self.max.x - ox;
            } else {
                let ox = self.min.x;
                self.min.x = other.max.x + 0.0001;
                self.max.x += self.min.x - ox;
            }
        }
        if dir.y != 0.0 {
            if dir.y > 0.0 {
                let oy = self.max.y;
                self.max.y = other.min.y - 0.0001;
                self.min.y += self.max.y - oy;
            } else {
                let oy = self.min.y;
                self.min.y = other.max.y + 0.0001;
                self.max.y += self.min.y - oy;
            }
        }
        if dir.z != 0.0 {
            if dir.z > 0.0 {
                let oz = self.max.z;
                self.max.z = other.min.z - 0.0001;
                self.min.z += self.max.z - oz;
            } else {
                let oz = self.min.z;
                self.min.z = other.max.z + 0.0001;
                self.max.z += self.min.z - oz;
            }
        }
        self
    }
}

pub fn resolve_textures(texture: &Texture, width: f32, height: f32, offset_x: f32, offset_y: f32) -> [Option<Texture>; 6] {
    [
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[0][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[1][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[1][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[2][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[2][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[3][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[3][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[4][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[4][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[5][0]) / (texture.get_width() as f32), (offset_y + height * TEXTURE_MATRIX[5][1]) / (texture.get_height() as f32), width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
    ]
}
