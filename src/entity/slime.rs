use super::{Bounds, GameInfo, Light, Position, Rotation, Velocity};
use crate::ecs;
use crate::ecs::Entity;
use crate::entity::{resolve_textures, CustomEntityRenderer, EntityType};
use crate::render;
use crate::render::model;
use crate::render::Renderer;
use crate::world;
use cgmath::{self, Decomposed, Matrix4, Point3, Quaternion, Rad, Rotation3, Vector3};
use collision::Aabb3;

pub struct SlimeModel {
    model: Option<model::ModelKey>,
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
    Body = 0,
    Eyes = 1,
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
        e: Entity,
    ) {
        use std::f32::consts::PI;
        use std::f64::consts::PI as PI64;
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

    #[allow(clippy::eq_op)]
    fn entity_added(
        &self,
        m: &mut ecs::Manager,
        e: ecs::Entity,
        _: &world::World,
        renderer: &mut render::Renderer,
    ) {
        let slime_model = m.get_component_mut(e, self.slime_model).unwrap();
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

        slime_model.model = Some(renderer.model.create_model(
            model::DEFAULT,
            vec![
                body_verts, eye_verts,
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
