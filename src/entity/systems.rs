use super::*;
use crate::entity::player::PlayerMovement;
use crate::particle::block_break_effect::{BlockBreakEffect, BlockEffectData};
use crate::server::{ConnResource, InventoryContextResource, RendererResource, WorldResource};
use crate::shared::Position as BPos;
use crate::world::World;
use cgmath::InnerSpace;
use leafish_blocks::Block;
use leafish_protocol::protocol;
use leafish_protocol::protocol::packet;
use parking_lot::RwLock;
use shared::Direction;

pub fn apply_velocity(mut query: Query<(&mut Position, &Velocity), Without<PlayerMovement>>) {
    // Player's handle their own physics
    for (mut pos, vel) in query.iter_mut() {
        pos.position += vel.velocity;
    }
}

pub fn apply_gravity(mut query: Query<&mut Velocity, (Without<PlayerMovement>, With<Gravity>)>) {
    // Player's handle their own physics
    for mut vel in query.iter_mut() {
        vel.velocity.y -= 0.03;
        if vel.velocity.y < -0.3 {
            vel.velocity.y = -0.3;
        }
    }
}

pub fn update_last_position(mut query: Query<&mut Position>) {
    for mut pos in query.iter_mut() {
        pos.moved = (pos.position - pos.last_position).magnitude2() > 0.01;
        pos.last_position = pos.position;
    }
}

pub fn lerp_position(game_info: Res<GameInfo>, mut query: Query<(&mut Position, &TargetPosition)>) {
    let delta = game_info.delta.min(5.0);
    for (mut pos, target_pos) in query.iter_mut() {
        pos.position =
            pos.position + (target_pos.position - pos.position) * delta * target_pos.lerp_amount;
        let len = (pos.position - target_pos.position).magnitude2();
        if !(0.001..=100.0 * 100.0).contains(&len) {
            pos.position = target_pos.position;
        }
    }
}

pub fn lerp_rotation(
    game_info: Res<GameInfo>,
    mut query: Query<(&mut Rotation, &mut TargetRotation)>,
) {
    use std::f64::consts::PI;
    let delta = game_info.delta.min(5.0);
    for (mut rot, mut target_rot) in query.iter_mut() {
        target_rot.yaw = (PI * 2.0 + target_rot.yaw) % (PI * 2.0);
        target_rot.pitch = (PI * 2.0 + target_rot.pitch) % (PI * 2.0);

        let mut delta_yaw = target_rot.yaw - rot.yaw;
        let mut delta_pitch = target_rot.pitch - rot.pitch;

        if delta_yaw.abs() > PI {
            delta_yaw = (PI - delta_yaw.abs()) * delta_yaw.signum();
        }
        if delta_pitch.abs() > PI {
            delta_pitch = (PI - delta_pitch.abs()) * delta_pitch.signum();
        }

        rot.yaw += delta_yaw * 0.2 * delta;
        rot.pitch += delta_pitch * 0.2 * delta;
        rot.yaw = (PI * 2.0 + rot.yaw) % (PI * 2.0);
        rot.pitch = (PI * 2.0 + rot.pitch) % (PI * 2.0);
    }
}

pub fn light_entity(world: Res<WorldResource>, mut query: Query<(&Position, &Bounds, &mut Light)>) {
    let world = &world.0;
    for (pos, bounds, mut light) in query.iter_mut() {
        let mut count = 0.0;
        let mut block_light = 0.0;
        let mut sky_light = 0.0;

        let min_x = (pos.position.x + bounds.bounds.min.x).floor() as i32;
        let min_y = (pos.position.y + bounds.bounds.min.y).floor() as i32;
        let min_z = (pos.position.z + bounds.bounds.min.z).floor() as i32;
        let max_x = (pos.position.x + bounds.bounds.max.x).ceil() as i32 + 1;
        let max_y = (pos.position.y + bounds.bounds.max.y).ceil() as i32 + 1;
        let max_z = (pos.position.z + bounds.bounds.max.z).ceil() as i32 + 1;

        let length = (bounds.bounds.max - bounds.bounds.min).magnitude() as f32;

        for y in min_y..max_y {
            for z in min_z..max_z {
                for x in min_x..max_x {
                    let dist = length
                        - (((x as f32 + 0.5) - pos.position.x as f32).powi(2)
                            + ((y as f32 + 0.5) - pos.position.y as f32).powi(2)
                            + ((z as f32 + 0.5) - pos.position.z as f32).powi(2))
                        .sqrt()
                        .min(length);
                    let dist = dist / length;
                    count += dist;
                    block_light += world.get_block_light(BPos::new(x, y, z)) as f32 * dist;
                    sky_light += world.get_sky_light(BPos::new(x, y, z)) as f32 * dist;
                }
            }
        }
        if count <= 0.01 {
            light.block_light = 0.0;
            light.sky_light = 0.0;
        } else {
            light.block_light = block_light / count;
            light.sky_light = sky_light / count;
        }
    }
}

pub fn apply_digging(
    renderer: Res<RendererResource>,
    world: Res<WorldResource>,
    conn: Res<ConnResource>,
    inventory: Res<InventoryContextResource>,
    commands: Commands,
    mut query: Query<(&MouseButtons, &mut Digging)>,
    mut effect_query: Query<&mut BlockBreakEffect>,
) {
    use crate::server::target::{test_block, trace_ray};
    use cgmath::EuclideanSpace;

    let world = &world.0;
    let renderer = &renderer.0;
    let conn = &conn.0;
    let inventory = &inventory.0;

    let target = trace_ray(
        world.as_ref(),
        4.0,
        renderer.camera.lock().pos.to_vec(),
        renderer.view_vector.lock().cast().unwrap(),
        test_block,
    );

    let tool = {
        let inventory = inventory.read();
        let hotbar_index = inventory.hotbar_index;
        let inventory = inventory.base_slots.read();
        let item = inventory.get_item(27 + hotbar_index as u16);
        item.and_then(|i| i.material.as_tool())
    };

    let mut system = ApplyDigging::new(target, conn.clone(), commands, tool);

    for (mouse_buttons, mut digging) in query.iter_mut() {
        if let Some(effect) = digging.effect {
            if let Ok(mut effect) = effect_query.get_mut(effect) {
                system.update(
                    mouse_buttons,
                    digging.as_mut(),
                    Some(effect.as_mut()),
                    world,
                );
            }
        }
        system.update(mouse_buttons, digging.as_mut(), None, world);
    }
}

struct ApplyDigging<'w, 's> {
    target: Option<(shared::Position, Block, Direction, Vector3<f64>)>,
    conn: Arc<RwLock<Option<protocol::Conn>>>,
    commands: Commands<'w, 's>,
    tool: Option<block::Tool>,
}

impl ApplyDigging<'_, '_> {
    pub fn new<'a, 'b>(
        target: Option<(shared::Position, Block, Direction, Vector3<f64>)>,
        conn: Arc<RwLock<Option<protocol::Conn>>>,
        commands: Commands<'a, 'b>,
        tool: Option<block::Tool>,
    ) -> ApplyDigging<'a, 'b> {
        ApplyDigging {
            target,
            conn,
            commands,
            tool,
        }
    }

    fn update(
        &mut self,
        mouse_buttons: &MouseButtons,
        digging: &mut Digging,
        effect: Option<&mut BlockBreakEffect>,
        world: &Arc<World>,
    ) {
        // Move the previous current value into last, and then calculate the
        // new current value.
        std::mem::swap(&mut digging.last, &mut digging.current);
        digging.current = self.next_state(&digging.last, mouse_buttons, self.target);

        // Send required digging packets
        match (&digging.last, &mut digging.current) {
            // Start the new digging operation.
            (None, Some(current)) => self.start_digging(current, &mut digging.effect),
            // Cancel the previous digging operation.
            (Some(last), None) if !last.finished => self.abort_digging(last, &mut digging.effect),
            // Move to digging a new block
            (Some(last), Some(current)) if last.position != current.position => {
                // Cancel the previous digging operation.
                if !current.finished {
                    self.abort_digging(last, &mut digging.effect);
                }
                // Start the new digging operation.
                self.start_digging(current, &mut digging.effect);
            }
            // Finish the new digging operation.
            (Some(_), Some(current)) if current.is_finished(&self.tool) => {
                current.finished = true;
                self.finish_digging(current, &mut digging.effect, world);
            }
            _ => {}
        }

        if let Some(current) = &digging.current {
            // Update the block break animation progress.
            if let Some(effect) = effect {
                effect.update_ratio(current.get_ratio(&self.tool));
            }
            self.swing_arm();
        }
    }

    fn next_state(
        &self,
        last: &Option<DiggingState>,
        mouse_buttons: &MouseButtons,
        target: Option<(
            shared::Position,
            block::Block,
            shared::Direction,
            Vector3<f64>,
        )>,
    ) -> Option<DiggingState> {
        if !mouse_buttons.left {
            return None;
        }

        match (last, target) {
            // Started digging
            (None, Some((position, block, face, _))) => Some(DiggingState {
                block,
                face,
                position,
                start: std::time::Instant::now(),
                finished: false,
            }),
            (Some(current), Some((position, block, face, ..))) => {
                if position == current.position {
                    // Continue digging
                    last.clone()
                } else {
                    // Start digging a different block.
                    Some(DiggingState {
                        block,
                        face,
                        position,
                        start: std::time::Instant::now(),
                        finished: false,
                    })
                }
            }
            // Not pointing at any target
            (_, None) => None,
        }
    }

    fn start_digging(&mut self, state: &DiggingState, effect: &mut Option<Entity>) {
        let mut entity = self.commands.spawn_empty();
        let pos = state.position;
        entity.insert(BlockEffectData {
            position: Vector3::new(pos.x as f64, pos.y as f64, pos.z as f64),
            status: -1,
        });
        entity.insert(crate::particle::ParticleType::BlockBreak);
        effect.replace(entity.id());

        self.send_digging(state, packet::DigType::StartDestroyBlock);
    }

    fn abort_digging(&mut self, state: &DiggingState, effect: &mut Option<Entity>) {
        if let Some(effect) = effect.take() {
            self.commands.entity(effect).despawn();
        }
        self.send_digging(state, packet::DigType::AbortDestroyBlock);
    }

    fn finish_digging(
        &mut self,
        state: &DiggingState,
        effect: &mut Option<Entity>,
        world: &Arc<World>,
    ) {
        if let Some(effect) = effect.take() {
            self.commands.entity(effect).despawn();
        }
        world.set_block(state.position, block::Block::Air {});

        self.send_digging(state, packet::DigType::FinishDestroyBlock);
    }

    fn send_digging(&self, state: &DiggingState, status: packet::DigType) {
        let mut conn = self.conn.write();
        packet::send_digging(
            conn.as_mut().unwrap(),
            status,
            state.position,
            state.face.index() as u8,
        )
        .unwrap();
    }

    fn swing_arm(&self) {
        let mut conn = self.conn.write();
        packet::send_arm_swing(conn.as_mut().unwrap(), packet::Hand::MainHand).unwrap();
    }
}
