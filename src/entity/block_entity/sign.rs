use crate::ecs;
use crate::ecs::SystemExecStage;
use crate::format::{self, Component};
use crate::render;
use crate::render::model::{self, FormatState};
use crate::render::Renderer;
use crate::shared::{Direction, Position};
use crate::world::block::Block;
use bevy_ecs::prelude::*;
use std::sync::Arc;

pub fn add_systems(_m: &mut ecs::Manager, _parallel: &mut SystemStage, sync: &mut SystemStage) {
    sync.add_system(
        render_sign
            .system()
            .label(SystemExecStage::Render)
            .after(SystemExecStage::Normal),
    )
    .add_system(
        on_add_sign
            .system()
            .label(SystemExecStage::Render)
            .after(SystemExecStage::Normal),
    );
}

pub fn init_entity(m: &mut ecs::Manager, e: Entity) {
    m.world.get_entity_mut(e).unwrap().insert(SignInfo {
        model: None,
        lines: [
            Component::new(format::ComponentType::new("", None)),
            Component::new(format::ComponentType::new("", None)),
            Component::new(format::ComponentType::new("", None)),
            Component::new(format::ComponentType::new("", None)),
        ],
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        has_stand: false,
        rotation: 0.0,
        dirty: false,
    });
}

#[derive(Component)]
pub struct SignInfo {
    model: Option<model::ModelHandle>,

    pub lines: [format::Component; 4],
    pub dirty: bool,

    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    has_stand: bool,
    rotation: f64,
}

pub fn render_sign(
    renderer: Res<Arc<Renderer>>,
    world: Res<Arc<crate::world::World>>,
    mut query: Query<(&mut SignInfo, &Position)>,
) {
    for (mut info, position) in query.iter_mut() {
        if info.dirty {
            remove_sign(&mut info);
            add_sign(renderer.clone(), world.clone(), &mut info, position);
        }
        if let Some(model) = &info.model {
            let renderer = renderer.clone();
            let mut models = renderer.models.lock();
            let mdl = models.get_model(model).unwrap();
            mdl.block_light = world.get_block_light(*position) as f32;
            mdl.sky_light = world.get_sky_light(*position) as f32;
        }
    }
}

pub fn on_add_sign(
    renderer: Res<Arc<Renderer>>,
    world: Res<Arc<crate::world::World>>,
    mut query: Query<(&mut SignInfo, &Position), Added<SignInfo>>,
) {
    for (mut info, position) in query.iter_mut() {
        add_sign(renderer.clone(), world.clone(), &mut info, position);
    }
}

fn add_sign(
    renderer: Arc<Renderer>,
    world: Arc<crate::world::World>,
    info: &mut SignInfo,
    position: &Position,
) {
    use cgmath::{Decomposed, Matrix4, Quaternion, Rad, Rotation3, Vector3};
    use std::f64::consts::PI;
    info.dirty = false;
    let block = world.get_block(*position);
    match block {
        Block::OakWallSign { facing, .. }
        | Block::SpruceWallSign { facing, .. }
        | Block::BirchWallSign { facing, .. }
        | Block::AcaciaWallSign { facing, .. }
        | Block::JungleWallSign { facing, .. }
        | Block::DarkOakWallSign { facing, .. }
        | Block::MangroveWallSign { facing, .. }
        | Block::CrimsonWallSign { facing, .. }
        | Block::WarpedWallSign { facing, .. } => {
            info.offset_z = 7.5 / 16.0;
            match facing {
                Direction::North => {}
                Direction::South => info.rotation = PI,
                Direction::West => info.rotation = PI / 2.0,
                Direction::East => info.rotation = -PI / 2.0,
                _ => unreachable!(),
            }
        }
        Block::OakSign { rotation, .. }
        | Block::SpruceSign { rotation, .. }
        | Block::BirchSign { rotation, .. }
        | Block::AcaciaSign { rotation, .. }
        | Block::JungleSign { rotation, .. }
        | Block::DarkOakSign { rotation, .. }
        | Block::MangroveSign { rotation, .. }
        | Block::CrimsonSign { rotation, .. }
        | Block::WarpedSign { rotation, .. } => {
            info.offset_y = 5.0 / 16.0;
            info.has_stand = true;
            info.rotation = -(rotation as f64 / 16.0) * PI * 2.0 + PI;
        }
        _ => return,
    }

    let wood_type = match block {
        Block::OakWallSign { .. } | Block::OakSign { .. } => "oak",
        Block::SpruceWallSign { .. } | Block::SpruceSign { .. } => "spruce",
        Block::BirchWallSign { .. } | Block::BirchSign { .. } => "birch",
        Block::AcaciaWallSign { .. } | Block::AcaciaSign { .. } => "acacia",
        Block::JungleWallSign { .. } | Block::JungleSign { .. } => "jungle",
        Block::DarkOakWallSign { .. } | Block::DarkOakSign { .. } => "dark_oak",
        Block::MangroveWallSign { .. } | Block::MangroveSign { .. } => "mangrove",
        Block::CrimsonWallSign { .. } | Block::CrimsonSign { .. } => "crimson",
        Block::WarpedWallSign { .. } | Block::WarpedSign { .. } => "warped",
        _ => "oak",
    };

    let path = format!("entity/signs/{wood_type}");
    let tex = render::Renderer::get_texture(renderer.get_textures_ref(), &path);

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
        let mut state = FormatState {
            width: 0.0,
            offset: 0.0,
            text: Vec::new(),
            renderer: renderer.clone(),
            y_scale: Y_SCALE,
            x_scale: X_SCALE,
        };
        state.build(line, Some(format::Color::Black));
        let width = state.width;
        // Center align text
        for vert in &mut state.text {
            vert.x += width * 0.5;
            vert.y -= (Y_SCALE + 0.4 / 16.0) * (i as f32);
        }
        verts.extend_from_slice(&state.text);
    }
    let mut models = renderer.models.lock();
    let model = models.create_model(model::DEFAULT, vec![verts], renderer.clone());

    let mdl = models.get_model(&model).unwrap();
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
    drop(models); // if we don't do this, we would get a deadlock
                  // FIXME: Cleanup all the manual drops with seperate spans

    info.model.replace(model); // TODO: This can cause a deadlock, check why!
}

fn remove_sign(info: &mut SignInfo) {
    info.model.take();
}
