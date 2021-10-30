use super::*;
use crate::entity::player::PlayerMovement;
use crate::shared::Position as BPos;
use cgmath::InnerSpace;

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

pub fn light_entity(
    world: Res<Arc<crate::world::World>>,
    mut query: Query<(&Position, &Bounds, &mut Light)>,
) {
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
