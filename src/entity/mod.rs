pub mod block_entity;
pub mod player;

use crate::ecs;
use cgmath::Vector3;
use collision::Aabb3;
use crate::ecs::{Manager, System, Filter, Entity};
use crate::render::{Renderer, Texture};
use crate::world::World;
use lazy_static::lazy_static;
use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;
use crate::entity::slime::SlimeRenderer;
use parking_lot::Mutex;
use std::borrow::BorrowMut;
use crate::entity::player::PlayerRenderer;
use crate::entity::zombie::ZombieRenderer;

mod systems;
pub mod slime;
pub mod zombie;

// TODO: There may be wrong entries in this!
static TEXTURE_MATRIX: [[[f32; 3]; 6]; 2] = [
    [
        [0.0, 1.0, 0.0], // OR 0 1 0 [1.0, 0.0, 1.0], // OR 0 1 0
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 1.0], // OR 1 0 1 [0.0, 1.0, 0.0], // OR 1 0 1
    ],
    [
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ],
];


/*
resolve_textures(&tex, 8.0, 12.0, 4.0, 16.0, 16.0) // width, height, depth...
srel!(28.0, 16.0, 8.0, 4.0),  // Down  | 1 0 1 | 0 0 0 OR 0 1 0 | 0 0 0
srel!(20.0, 16.0, 8.0, 4.0),  // Up    | 0 0 1 | 0 0 0
srel!(20.0, 20.0, 8.0, 12.0), // North | 0 0 1 | 0 0 1
srel!(32.0, 20.0, 8.0, 12.0), // South | 0 1 1 | 0 0 1
srel!(16.0, 20.0, 4.0, 12.0), // West  | 0 0 0 | 0 0 1
srel!(28.0, 20.0, 4.0, 12.0), // East  | 0 1 0 | 0 0 1 OR 1 0 1 | 0 0 1
    [1.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 1.0],
    [2.0, 0.0, 0.0, 1.0],
    [2.0, 0.0, 0.0, 1.0],
    [0.0, 0.0, 0.0, 1.0],
*/

pub fn add_systems(m: &mut ecs::Manager) {
    let sys = systems::UpdateLastPosition::new(m);
    m.add_system(sys);

    player::add_systems(m);

    let sys = systems::ApplyVelocity::new(m);
    m.add_system(sys);
    let sys = systems::ApplyGravity::new(m);
    m.add_system(sys);
    let sys = systems::LerpPosition::new(m);
    m.add_render_system(sys);
    let sys = systems::LerpRotation::new(m);
    m.add_render_system(sys);
    let sys = systems::LightEntity::new(m);
    m.add_render_system(sys);

    block_entity::add_systems(m);
}

/// Location of an entity in the world.
#[derive(Debug)]
pub struct Position {
    pub position: Vector3<f64>,
    pub last_position: Vector3<f64>,
    pub moved: bool,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Position {
        Position {
            position: Vector3::new(x, y, z),
            last_position: Vector3::new(x, y, z),
            moved: false,
        }
    }

    pub fn zero() -> Position {
        Position::new(0.0, 0.0, 0.0)
    }
}

#[derive(Debug)]
pub struct TargetPosition {
    pub position: Vector3<f64>,
    pub lerp_amount: f64,
}

impl TargetPosition {
    pub fn new(x: f64, y: f64, z: f64) -> TargetPosition {
        TargetPosition {
            position: Vector3::new(x, y, z),
            lerp_amount: 0.2,
        }
    }

    pub fn zero() -> TargetPosition {
        TargetPosition::new(0.0, 0.0, 0.0)
    }
}

/// Velocity of an entity in the world.
#[derive(Debug)]
pub struct Velocity {
    pub velocity: Vector3<f64>,
}

impl Velocity {
    pub fn new(x: f64, y: f64, z: f64) -> Velocity {
        Velocity {
            velocity: Vector3::new(x, y, z),
        }
    }

    pub fn zero() -> Velocity {
        Velocity::new(0.0, 0.0, 0.0)
    }
}

/// Rotation of an entity in the world
#[derive(Debug)]
pub struct Rotation {
    pub yaw: f64,
    pub pitch: f64,
}

impl Rotation {
    pub fn new(yaw: f64, pitch: f64) -> Rotation {
        Rotation { yaw, pitch }
    }

    pub fn zero() -> Rotation {
        Rotation::new(0.0, 0.0)
    }
}
#[derive(Debug)]
pub struct TargetRotation {
    pub yaw: f64,
    pub pitch: f64,
}

impl TargetRotation {
    pub fn new(yaw: f64, pitch: f64) -> TargetRotation {
        TargetRotation { yaw, pitch }
    }

    pub fn zero() -> TargetRotation {
        TargetRotation::new(0.0, 0.0)
    }
}

#[derive(Default)]
pub struct Gravity {
    pub on_ground: bool,
}

impl Gravity {
    pub fn new() -> Gravity {
        Default::default()
    }
}

pub struct Bounds {
    pub bounds: Aabb3<f64>,
}

impl Bounds {
    pub fn new(bounds: Aabb3<f64>) -> Bounds {
        Bounds { bounds }
    }
}

#[derive(Default)]
pub struct GameInfo {
    pub delta: f64,
}

impl GameInfo {
    pub fn new() -> GameInfo {
        Default::default()
    }
}

#[derive(Default)]
pub struct Light {
    pub block_light: f32,
    pub sky_light: f32,
}

impl Light {
    pub fn new() -> Light {
        Default::default()
    }
}

pub struct EntityRenderer {

    filter: ecs::Filter,
    position: ecs::Key<Position>,
    rotation: ecs::Key<Rotation>,
    entity_type: ecs::Key<EntityType>,

}

impl EntityRenderer {

    pub fn new(manager: &mut Manager) -> Self {
        let position = manager.get_key();
        let rotation = manager.get_key();
        let entity_type = manager.get_key();
        EntityRenderer {
            filter: ecs::Filter::new()
                .with(position)
                .with(rotation)
                .with(entity_type),
            position,
            rotation,
            entity_type,
        }
    }

}

impl System for EntityRenderer {
    fn filter(&self) -> &Filter {
        &self.filter
    }

    fn update(&mut self, m: &mut Manager, world: &World, renderer: &mut Renderer, focused: bool, dead: bool) {
        for e in m.find(&self.filter) {
            /*let position = m.get_component_mut(e, self.position).unwrap();
            let rotation = m.get_component_mut(e, self.rotation).unwrap();*/
            let entity_type = m.get_component(e, self.entity_type).unwrap();
            let mut c_renderer = entity_type.get_renderer();
            c_renderer.update(m, world, renderer, focused, dead, e);
        }
    }

    fn entity_added(&mut self, m: &mut Manager, e: Entity, world: &World, renderer: &mut Renderer) {
        let entity_type = m.get_component(e, self.entity_type).unwrap();
        let mut c_renderer = entity_type.get_renderer();
        c_renderer.entity_added(m, e, world, renderer);
    }

    fn entity_removed(&mut self, m: &mut Manager, e: Entity, world: &World, renderer: &mut Renderer) {
        let entity_type = m.get_component(e, self.entity_type).unwrap();
        let mut c_renderer = entity_type.get_renderer();
        c_renderer.entity_removed(m, e, world, renderer);
    }
}

pub trait CustomEntityRenderer {

    fn update(&self,
              manager: &mut Manager,
              world: &World,
              renderer: &mut Renderer,
              focused: bool,
              dead: bool,
              entity: Entity);

    fn entity_added(&self,
                    manager: &mut ecs::Manager,
                    entity: ecs::Entity,
                    world: &World,
                    renderer: &mut Renderer);

    fn entity_removed(&self,
                      manager: &mut Manager,
                      entity: ecs::Entity,
                      world: &World,
                      renderer: &mut Renderer);

}

pub struct NOOPEntityRenderer {



}

impl CustomEntityRenderer for NOOPEntityRenderer {
    fn update(&self, manager: &mut Manager, world: &World, renderer: &mut Renderer, focused: bool, dead: bool, entity: Entity) {

    }

    fn entity_added(&self, manager: &mut Manager, entity: Entity, world: &World, renderer: &mut Renderer) {

    }

    fn entity_removed(&self, manager: &mut Manager, entity: Entity, world: &World, renderer: &mut Renderer) {

    }
}

#[derive(Eq, PartialEq, Hash)]
pub enum EntityType {

    Slime,
    Player,
    Bat,
    Horse,
    Zombie,

}

lazy_static! {
    static ref ENTITY_RENDERERS: Arc<DashMap<EntityType, Arc<dyn CustomEntityRenderer + Send + Sync>>> = Arc::new(DashMap::new());
    static ref NOOP_RENDERER: Arc<dyn CustomEntityRenderer + Send + Sync> = Arc::new(NOOPEntityRenderer {});
}

impl EntityType {

    pub fn init(manager: &mut Manager) {
        ENTITY_RENDERERS.insert(EntityType::Slime, Arc::new(SlimeRenderer::new(manager)));
        ENTITY_RENDERERS.insert(EntityType::Player, Arc::new(PlayerRenderer::new(manager)));
        ENTITY_RENDERERS.insert(EntityType::Zombie, Arc::new(ZombieRenderer::new(manager)));
    }

    pub fn deinit() {
        ENTITY_RENDERERS.clear();
    }

    pub fn get_renderer(&self) -> Arc<dyn CustomEntityRenderer + Send + Sync> {
        ENTITY_RENDERERS.get(&self).map_or(NOOP_RENDERER.clone(), |x| x.value().clone())
    }

}

pub fn resolve_textures(texture: &Texture, width: f32, height: f32, depth: f32, offset_x: f32, offset_y: f32) -> [Option<Texture>; 6] {
    [
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][0][0] + height * TEXTURE_MATRIX[0][0][1] + depth * TEXTURE_MATRIX[0][0][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][0][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][1][0] + height * TEXTURE_MATRIX[0][1][1] + depth * TEXTURE_MATRIX[0][1][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][1][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][2][0] + height * TEXTURE_MATRIX[0][2][1] + depth * TEXTURE_MATRIX[0][2][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][2][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][3][0] + height * TEXTURE_MATRIX[0][3][1] + depth * TEXTURE_MATRIX[0][3][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][3][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][4][0] + height * TEXTURE_MATRIX[0][4][1] + depth * TEXTURE_MATRIX[0][4][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][4][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
        Some(texture.relative((offset_x + width * TEXTURE_MATRIX[0][5][0] + height * TEXTURE_MATRIX[0][5][1] + depth * TEXTURE_MATRIX[0][5][2]) / (texture.get_width() as f32),
                              (offset_y + depth * TEXTURE_MATRIX[1][5][2]) / (texture.get_height() as f32),
                              width / (texture.get_width() as f32), height / (texture.get_height() as f32))),
    ]
}
