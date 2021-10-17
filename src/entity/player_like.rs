use crate::entity::resolve_textures;
use crate::format;
use crate::render::model::{self, FormatState, Vertex};
use crate::render::{Renderer, Texture};
use std::sync::Arc;

pub enum PlayerLikeModelPart {
    Head = 0,
    Body = 1,
    LegLeft = 2,
    LegRight = 3,
    ArmLeft = 4,
    ArmRight = 5,
    NameTag = 6,
}

/*
fn update(
        m: &mut ecs::Manager,
        world: &world::World,
        renderer: &mut render::Renderer,
        e: Entity,
    position: &mut Position,
    rotation: &mut Rotation,
    light: &Light,
    delta: f32
    ) {
        use std::f32::consts::PI;
        use std::f64::consts::PI as PI64;
        let player_model = m.get_component_mut(e, self.player_model).unwrap();

        if let Some(pmodel) = player_model.model {
            let mdl = renderer.model.get_model(pmodel).unwrap();

            mdl.block_light = light.block_light;
            mdl.sky_light = light.sky_light;

            let offset = if player_model.first_person {
                let ox = (rotation.yaw - PI64 / 2.0).cos() * 0.25;
                let oz = -(rotation.yaw - PI64 / 2.0).sin() * 0.25;
                Vector3::new(
                    position.position.x as f32 - ox as f32,
                    -position.position.y as f32,
                    position.position.z as f32 - oz as f32,
                )
            } else {
                Vector3::new(
                    position.position.x as f32,
                    -position.position.y as f32,
                    position.position.z as f32,
                )
            };
            let offset_matrix = Matrix4::from(Decomposed {
                scale: 1.0,
                rot: Quaternion::from_angle_y(Rad(PI + rotation.yaw as f32)),
                disp: offset,
            });

            // TODO This sucks
            if player_model.has_name_tag {
                let ang = (position.position.x - renderer.camera.pos.x)
                    .atan2(position.position.z - renderer.camera.pos.z)
                    as f32;
                mdl.matrix[PlayerModelPart::NameTag as usize] = Matrix4::from(Decomposed {
                    scale: 1.0,
                    rot: Quaternion::from_angle_y(Rad(ang)),
                    disp: offset + Vector3::new(0.0, (-24.0 / 16.0) - 0.6, 0.0),
                });
            }

            mdl.matrix[PlayerModelPart::Head as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                scale: 1.0,
                rot: Quaternion::from_angle_x(Rad(-rotation.pitch as f32)),
                disp: Vector3::new(0.0, -12.0 / 16.0 - 12.0 / 16.0, 0.0),
            });
            mdl.matrix[PlayerModelPart::Body as usize] = offset_matrix
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

            mdl.matrix[PlayerModelPart::LegRight as usize] = offset_matrix
                * Matrix4::from(Decomposed {
                scale: 1.0,
                rot: Quaternion::from_angle_x(Rad(ang as f32)),
                disp: Vector3::new(2.0 / 16.0, -12.0 / 16.0, 0.0),
            });
            mdl.matrix[PlayerModelPart::LegLeft as usize] = offset_matrix
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

            if player_model.arm_time <= 0.0 {
                player_model.arm_time = 0.0;
            } else {
                player_model.arm_time -= delta;
            }

            mdl.matrix[PlayerModelPart::ArmRight as usize] = offset_matrix
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
                - ((7.5 - (player_model.arm_time - 7.5).abs()) / 7.5))
                as f32)));

            mdl.matrix[PlayerModelPart::ArmLeft as usize] = offset_matrix
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
    }*/

pub fn compute_player_model_components(
    tex: &Texture,
    name: &Option<String>,
    renderer: Arc<Renderer>,
) -> Vec<Vec<Vertex>> {
    // TODO: Replace this shit entirely!
    macro_rules! srel {
        ($x:expr, $y:expr, $w:expr, $h:expr) => {
            Some(tex.relative(
                ($x) / (tex.get_width() as f32),
                ($y) / (tex.get_height() as f32),
                ($w) / (tex.get_width() as f32),
                ($h) / (tex.get_height() as f32),
            ))
        };
    }

    let mut head_verts = vec![];
    model::append_box(
        &mut head_verts,
        -4.0 / 16.0,
        0.0,
        -4.0 / 16.0,
        8.0 / 16.0,
        8.0 / 16.0,
        8.0 / 16.0,
        resolve_textures(tex, 8.0, 8.0, 8.0, 0.0, 0.0),
    );
    model::append_box(
        &mut head_verts,
        -4.2 / 16.0,
        -0.2 / 16.0,
        -4.2 / 16.0,
        8.4 / 16.0,
        8.4 / 16.0,
        8.4 / 16.0,
        resolve_textures(tex, 8.0, 8.0, 8.0, 32.0, 0.0),
    );

    let mut body_verts = vec![];
    model::append_box(
        &mut body_verts,
        -4.0 / 16.0,
        -6.0 / 16.0,
        -2.0 / 16.0,
        8.0 / 16.0,
        12.0 / 16.0,
        4.0 / 16.0,
        resolve_textures(tex, 8.0, 12.0, 4.0, 16.0, 16.0),
    );
    model::append_box(
        &mut body_verts,
        -4.2 / 16.0,
        -6.2 / 16.0,
        -2.2 / 16.0,
        8.4 / 16.0,
        12.4 / 16.0,
        4.4 / 16.0,
        resolve_textures(tex, 8.0, 12.0, 4.0, 16.0, 16.0),
    );

    let mut part_verts = vec![vec![]; 4];

    for (i, offsets) in [
        [0.0, 16.0, 0.0, 32.0],   // Left leg
        [0.0, 16.0, 0.0, 32.0],   // Right Leg
        [40.0, 16.0, 40.0, 32.0], // Left arm
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
                srel!(ox + 8.0, oy + 0.0, 4.0, 4.0),   // Down
                srel!(ox + 4.0, oy + 0.0, 4.0, 4.0),   // Up
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
    }

    let mut name_verts = vec![];
    if name.is_some() {
        let mut state = FormatState {
            width: 0.0,
            offset: 0.0,
            text: Vec::new(),
            renderer,
            y_scale: 0.16,
            x_scale: 0.01,
        };
        let name = format::Component::new(format::ComponentType::new_with_color(
            name.as_ref().unwrap(),
            format::Color::Black,
        ));
        state.build(&name, None);
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
    }

    vec![
        head_verts,
        body_verts,
        part_verts[0].clone(),
        part_verts[1].clone(),
        part_verts[2].clone(),
        part_verts[3].clone(),
        name_verts,
    ]
}
