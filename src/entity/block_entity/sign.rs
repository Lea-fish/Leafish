use crate::ecs;
use crate::format::{self, Component};
use crate::render;
use crate::render::model::{self, FormatState};
use crate::shared::{Direction, Position};
use crate::world;
use crate::world::block::Block;
use bevy_ecs::prelude::*;
use crate::render::Renderer;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::ecs::SystemExecStage;

pub fn add_systems(m: &mut ecs::Manager, parallel: &mut SystemStage, sync: &mut SystemStage) {
    sync.add_system(render_sign.system().label(SystemExecStage::Render).after(SystemExecStage::Normal))
        .add_system(on_add_sign.system().label(SystemExecStage::Render).after(SystemExecStage::Normal))
        .add_system(on_sign_remove.system().label(SystemExecStage::RemoveHandling).after(SystemExecStage::Render));
}

pub fn init_entity(m: &mut ecs::Manager, e: Entity) {
    m.world.get_entity_mut(e).unwrap().insert(SignInfo {
        model: None,
        lines: [
            Component::Text(format::TextComponent::new("")),
            Component::Text(format::TextComponent::new("")),
            Component::Text(format::TextComponent::new("")),
            Component::Text(format::TextComponent::new("")),
        ],
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        has_stand: false,
        rotation: 0.0,
        dirty: false,
    });
}

pub struct SignInfo {
    model: Option<model::ModelKey>,

    pub lines: [format::Component; 4],
    pub dirty: bool,

    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    has_stand: bool,
    rotation: f64,
}

pub fn render_sign(renderer: Res<Arc<RwLock<Renderer>>>, world: Res<Arc<crate::world::World>>, mut query: Query<(&mut SignInfo, &Position)>) {
    for (mut info, position) in query.iter_mut() {
        if info.dirty {
            remove_sign(renderer.clone(), &mut *info);
            add_sign(renderer.clone(), world.clone(), &mut *info, position);
        }
        if let Some(model) = info.model {
            let renderer = renderer.clone();
            let mut renderer = renderer.write();
            let mdl = renderer.model.get_model(model).unwrap();
            mdl.block_light = world.get_block_light(*position) as f32;
            mdl.sky_light = world.get_sky_light(*position) as f32;
        }
    }
}

pub fn on_add_sign(renderer: Res<Arc<RwLock<Renderer>>>, world: Res<Arc<crate::world::World>>, mut query: Query<(&mut SignInfo, &Position), (Added<SignInfo>)>) {
   for (mut info, position) in query.iter_mut() {
       add_sign(renderer.clone(), world.clone(), &mut *info, position);
   }
}

pub fn on_sign_remove(renderer: Res<Arc<RwLock<Renderer>>>, _removed: RemovedComponents<SignInfo>, mut query: Query<(&mut SignInfo)>) {
    // TODO: Fix this!
    /*for (mut info) in query.iter_mut() {
        remove_sign(renderer.clone(), &mut *info);
    }*/
}

fn add_sign(renderer: Arc<RwLock<Renderer>>, world: Arc<crate::world::World>, info: &mut SignInfo, position: &Position) {
    use cgmath::{Decomposed, Matrix4, Quaternion, Rad, Rotation3, Vector3};
    use std::f64::consts::PI;
    info.dirty = false;
    match world.get_block(*position) {
        Block::WallSign { facing, .. } => {
            info.offset_z = 7.5 / 16.0;
            match facing {
                Direction::North => {}
                Direction::South => info.rotation = PI,
                Direction::West => info.rotation = PI / 2.0,
                Direction::East => info.rotation = -PI / 2.0,
                _ => unreachable!(),
            }
        }
        Block::StandingSign { rotation, .. } => {
            info.offset_y = 5.0 / 16.0;
            info.has_stand = true;
            info.rotation = -(rotation.data() as f64 / 16.0) * PI * 2.0 + PI;
        }
        _ => return,
    }
    let tex = render::Renderer::get_texture(renderer.clone().write().get_textures_ref(), "entity/sign");

    macro_rules! rel {
            ($x:expr, $y:expr, $w:expr, $h:expr) => {
                Some(tex.relative(($x) / 64.0, ($y) / 32.0, ($w) / 64.0, ($h) / 32.0))
            };
        }

    let mut verts = vec![];
    // Backboard
    model::append_box(
        &mut verts,
        -0.5,
        -4.0 / 16.0,
        -0.5 / 16.0,
        1.0,
        8.0 / 16.0,
        1.0 / 16.0,
        [
            rel!(26.0, 0.0, 24.0, 2.0),  // Down
            rel!(2.0, 0.0, 24.0, 2.0),   // Up
            rel!(2.0, 2.0, 24.0, 12.0),  // North
            rel!(26.0, 2.0, 24.0, 12.0), // South
            rel!(0.0, 2.0, 2.0, 12.0),   // West
            rel!(50.0, 2.0, 2.0, 12.0),  // East
        ],
    );
    if info.has_stand {
        model::append_box(
            &mut verts,
            -0.5 / 16.0,
            -0.25 - 9.0 / 16.0,
            -0.5 / 16.0,
            1.0 / 16.0,
            9.0 / 16.0,
            1.0 / 16.0,
            [
                rel!(4.0, 14.0, 2.0, 2.0),  // Down
                rel!(2.0, 14.0, 2.0, 2.0),  // Up
                rel!(2.0, 16.0, 2.0, 12.0), // North
                rel!(6.0, 16.0, 2.0, 12.0), // South
                rel!(0.0, 16.0, 2.0, 12.0), // West
                rel!(4.0, 16.0, 2.0, 12.0), // East
            ],
        );
    }

    for (i, line) in info.lines.iter().enumerate() {
        const Y_SCALE: f32 = (6.0 / 16.0) / 4.0;
        const X_SCALE: f32 = Y_SCALE / 16.0;
        let renderer = renderer.clone();
        let mut renderer = renderer.write();
        let mut state = FormatState {
            width: 0.0,
            offset: 0.0,
            text: Vec::new(),
            renderer: &mut renderer,
            y_scale: Y_SCALE,
            x_scale: X_SCALE,
        };
        state.build(line, format::Color::Black);
        let width = state.width;
        // Center align text
        for vert in &mut state.text {
            vert.x += width * 0.5;
            vert.y -= (Y_SCALE + 0.4 / 16.0) * (i as f32);
        }
        verts.extend_from_slice(&state.text);
    }

    let renderer = renderer.clone();
    let mut renderer = renderer.write();
    println!("create sign model!");
    let model = renderer.model.create_model(model::DEFAULT, vec![verts]);

    let mdl = renderer.model.get_model(model).unwrap();
    mdl.radius = 2.0;
    mdl.x = position.x as f32 + 0.5;
    mdl.y = position.y as f32 + 0.5;
    mdl.z = position.z as f32 + 0.5;
    mdl.matrix[0] = Matrix4::from(Decomposed {
        scale: 1.0,
        rot: Quaternion::from_angle_y(Rad(info.rotation as f32)),
        disp: Vector3::new(
            position.x as f32 + 0.5,
            -position.y as f32 - 0.5,
            position.z as f32 + 0.5,
        ),
    }) * Matrix4::from_translation(Vector3::new(
        info.offset_x as f32,
        -info.offset_y as f32,
        info.offset_z as f32,
    ));

    info.model = Some(model);
}

fn remove_sign(renderer: Arc<RwLock<Renderer>>, info: &mut SignInfo) {
    if let Some(model) = info.model {
        renderer.clone().write().model.remove_model(&model);
    }
    info.model = None;
}
