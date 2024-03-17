pub mod block_entity;
pub mod player;

use crate::ecs::{Manager, SystemExecStage};
use crate::entity::slime::SlimeModel;
use crate::entity::zombie::ZombieModel;
use crate::render::Texture;
use crate::world::block;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;
use block::MiningTime;
use cgmath::Vector3;
use collision::Aabb3;
use std::sync::Arc;

pub mod player_like;
pub mod slime;
mod systems;
pub mod versions;
pub mod zombie;

// TODO: There may be wrong entries in this!
// 1.0, 1.0, 0.0 | 0.0, 0.0, 0.0
static TEXTURE_MATRIX: [[[f32; 3]; 6]; 2] = [
    [
        [0.0, 1.0, 0.0], // OR (although the current one seems correct) 1 0 1 [1.0, 0.0, 1.0], // OR 1 0 1
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0], // south(back) - 0, 1, 1 | 1, 0, 1 - 0, 0, 1 displays the left half of the back (body) and the left side of the head
        [1.0, 0.0, 1.0], // left(west)
        [0.0, 0.0, 0.0], // right(east)
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

pub fn add_systems(sched: &mut Schedule, render_sched: &mut Schedule) {
    sched.add_systems(systems::update_last_position.in_set(SystemExecStage::Normal));

    player::add_systems(sched, render_sched);
    sched
        .add_systems(systems::apply_velocity.in_set(SystemExecStage::Normal))
        .add_systems(systems::apply_gravity.in_set(SystemExecStage::Normal))
        .add_systems(systems::apply_digging.in_set(SystemExecStage::Normal));

    render_sched /*sync*/
        .add_systems(
            systems::lerp_position
                .in_set(SystemExecStage::Render)
                .after(SystemExecStage::Normal),
        )
        .add_systems(
            systems::lerp_rotation
                .in_set(SystemExecStage::Render)
                .after(SystemExecStage::Normal),
        )
        .add_systems(
            systems::light_entity
                .in_set(SystemExecStage::Render)
                .after(SystemExecStage::Normal),
        );

    block_entity::add_systems(sched);
    crate::particle::block_break_effect::add_systems(sched);
}

/// Location of an entity in the world.
#[derive(Component, Debug)]
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

#[derive(Component, Debug)]
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
#[derive(Component, Debug)]
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
#[derive(Component, Debug)]
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
#[derive(Component, Debug)]
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

#[derive(Component, Default)]
pub struct Gravity {
    pub on_ground: bool,
}

impl Gravity {
    pub fn new() -> Gravity {
        Default::default()
    }
}

#[derive(Component)]
pub struct Bounds {
    pub bounds: Aabb3<f64>,
}

impl Bounds {
    pub fn new(bounds: Aabb3<f64>) -> Bounds {
        Bounds { bounds }
    }
}

#[derive(Default, Resource)]
pub struct GameInfo {
    pub delta: f64,
}

impl GameInfo {
    pub fn new() -> GameInfo {
        Default::default()
    }
}

#[derive(Component, Default)]
pub struct Light {
    pub block_light: f32,
    pub sky_light: f32,
}

impl Light {
    pub fn new() -> Light {
        Default::default()
    }
}

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum EntityType {
    DroppedItem,
    ExperienceOrb,
    LeashHitch,
    Painting,
    Arrow,
    Snowball,
    Fireball,
    SmallFireball,
    EnderPearl,
    EnderSignal,
    ThrownExpBottle,
    ItemFrame,
    WitherSkull,
    PrimedTnt,
    FallingBlock,
    Firework,
    TippedArrow,
    SpectralArrow,
    ShulkerBullet,
    DragonFireball,
    ArmorStand,
    MinecartCommand,
    Boat,
    Minecart,
    MinecartChest,
    MinecartFurnace,
    MinecartTnt,
    MinecartHopper,
    MinecartMobSpawner,
    Creeper,
    Skeleton,
    Spider,
    Giant,
    Zombie,
    Slime,
    Ghast,
    PigZombie,
    Enderman,
    CaveSpider,
    Silverfish,
    Blaze,
    MagmaCube,
    EnderDragon,
    Wither,
    Bat,
    Witch,
    Endermite,
    Guardian,
    Shulker,
    Pig,
    Sheep,
    Cow,
    Chicken,
    Squid,
    Wolf,
    MushroomCow,
    Snowman,
    Ocelot,
    IronGolem,
    Horse,
    Rabbit,
    PolarBear,
    Villager,
    EnderCrystal,
    SplashPotion,
    LingeringPotion,
    AreaEffectCloud,
    Egg,
    FishingHook,
    Lightning,
    Weather,
    Player,
    ComplexPart,
    Unknown,
    ElderGuardian,
    WitherSkeleton,
    Stray,
    Husk,
    ZombieVillager,
    SkeletonHorse,
    ZombieHorse,
    Donkey,
    Mule,
    EvokerFangs,
    Evoker,
    Vex,
    Vindicator,
    Llama,
    LlamaSpit,
    Illusioner,
    Parrot,
    Turtle,
    Phantom,
    Trident,
    Cod,
    Salmon,
    Pufferfish,
    TropicalFish,
    Drowned,
    Dolphin,
    Cat,
    Panda,
    Pillager,
    Ravager,
    TraderLlama,
    WanderingTrader,
    Fox,
    Bee,
    ZombifiedPiglin,
    Hoglin,
    Piglin,
    Strider,
    Zoglin,
    PiglinBrute,
}

impl EntityType {
    pub fn create_entity(
        &self,
        m: &mut Manager,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
    ) -> Option<Entity> {
        if self.supported() {
            let ret = self.create_entity_internally(m, x, y, z, yaw, pitch);
            self.create_model(m, ret);
            return Some(ret);
        }
        None
    }

    pub fn create_entity_custom_model(
        &self,
        m: &mut Manager,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
    ) -> Option<Entity> {
        if self.supported() {
            return Some(self.create_entity_internally(m, x, y, z, yaw, pitch));
        }
        None
    }

    fn create_entity_internally(
        &self,
        m: &mut Manager,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
        pitch: f64,
    ) -> Entity {
        let mut entity = m.world.spawn_empty();
        entity
            .insert(Position::new(x, y, z))
            .insert(Rotation::new(yaw, pitch))
            .insert(Velocity::new(0.0, 0.0, 0.0))
            .insert(TargetPosition::new(x, y, z))
            .insert(TargetRotation::new(yaw, pitch))
            .insert(Light::new())
            .insert(*self);
        entity.id()
    }

    fn create_model(&self, m: &mut Manager, entity: Entity) {
        match self {
            EntityType::Zombie => {
                m.world.entity_mut(entity).insert(ZombieModel::new(None));
            }
            EntityType::Slime => {
                m.world.entity_mut(entity).insert(SlimeModel::new(None));
            }
            _ => {}
        };
    }

    fn supported(&self) -> bool {
        matches!(self, EntityType::Zombie)
    }
}

pub fn resolve_textures(
    texture: &Texture,
    width: f32,
    height: f32,
    depth: f32,
    offset_x: f32,
    offset_y: f32,
) -> [Option<Texture>; 6] {
    [
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][0][0]
                + height * TEXTURE_MATRIX[0][0][1]
                + depth * TEXTURE_MATRIX[0][0][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][0][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][1][0]
                + height * TEXTURE_MATRIX[0][1][1]
                + depth * TEXTURE_MATRIX[0][1][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][1][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][2][0]
                + height * TEXTURE_MATRIX[0][2][1]
                + depth * TEXTURE_MATRIX[0][2][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][2][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][3][0]
                + height * TEXTURE_MATRIX[0][3][1]
                + depth * TEXTURE_MATRIX[0][3][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][3][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][4][0]
                + height * TEXTURE_MATRIX[0][4][1]
                + depth * TEXTURE_MATRIX[0][4][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][4][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
        Some(texture.relative(
            (offset_x
                + width * TEXTURE_MATRIX[0][5][0]
                + height * TEXTURE_MATRIX[0][5][1]
                + depth * TEXTURE_MATRIX[0][5][2])
                / (texture.get_width() as f32),
            (offset_y + depth * TEXTURE_MATRIX[1][5][2]) / (texture.get_height() as f32),
            width / (texture.get_width() as f32),
            height / (texture.get_height() as f32),
        )),
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiggingState {
    pub block: block::Block,
    pub position: shared::Position,
    pub face: shared::Direction,
    pub start: std::time::Instant,
    pub finished: bool,
}

#[derive(PartialEq, Eq)]
pub enum DiggingFinishState {
    Finished,
    FinishedInstant,
    NotFinished,
}

impl DiggingState {
    pub fn is_finished(&self, tool: &Option<block::Tool>) -> DiggingFinishState {
        // If marked as finished, we don't need to calculate the mining time
        // again.
        if self.finished {
            return DiggingFinishState::Finished;
        }

        let mining_time = self.block.get_mining_time(tool);
        match mining_time {
            block::MiningTime::Instant => DiggingFinishState::FinishedInstant,
            block::MiningTime::Time(mining_time) => {
                let finish_time = self.start + mining_time;
                if finish_time < std::time::Instant::now() {
                    DiggingFinishState::Finished
                } else {
                    DiggingFinishState::NotFinished
                }
            }
            block::MiningTime::Never => DiggingFinishState::NotFinished,
        }
    }

    pub fn get_ratio(&self, tool: &Option<block::Tool>) -> f32 {
        // If marked as finished, we don't need to calculate the mining time
        // again.
        if self.finished {
            return 1.0;
        }

        let mining_time = self.block.get_mining_time(tool);
        let mining_time = match mining_time {
            MiningTime::Instant => return 1.0,
            MiningTime::Never => return 0.0,
            MiningTime::Time(time) => time,
        };
        let now = std::time::Instant::now();
        let expected = now - self.start;
        let ratio = expected.as_secs_f32() / mining_time.as_secs_f32();
        ratio.min(1.0)
    }
}

#[derive(Component, Default)]
pub struct Digging {
    pub last: Option<DiggingState>,
    pub current: Option<DiggingState>,
    pub processed: bool,
    pub effect: Option<Entity>,
}

impl Digging {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Component, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
}

impl MouseButtons {
    pub fn new() -> Self {
        Default::default()
    }
}
