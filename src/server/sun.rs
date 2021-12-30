// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::render;
use crate::render::model;
use cgmath::{Decomposed, Matrix4, Quaternion, Rad, Rotation3, Vector3};
use std::sync::Arc;

pub struct SunModel {
    sun: model::ModelHandle,
    moon: model::ModelHandle,
    last_phase: i32,
}

const SIZE: f32 = 50.0;

impl SunModel {
    pub fn new(renderer: Arc<render::Renderer>) -> SunModel {
        SunModel {
            sun: SunModel::generate_sun(renderer.clone()),
            moon: SunModel::generate_moon(renderer, 0),
            last_phase: 0,
        }
    }

    pub fn tick(&mut self, renderer: Arc<render::Renderer>, world_time: f64, world_age: i64) {
        use std::f64::consts::PI;
        let phase = ((world_age / 24000) % 8) as i32;
        if phase != self.last_phase {
            self.moon = SunModel::generate_moon(renderer.clone(), phase);
            self.last_phase = phase;
        }

        let time = world_time / 12000.0;
        let ox = (time * PI).cos() * 300.0;
        let oy = (time * PI).sin() * 300.0;

        let mut models = renderer.models.lock();
        {
            let sun = models.get_model(&self.sun).unwrap();
            let camera = renderer.camera.lock();
            sun.matrix[0] = Matrix4::from(Decomposed {
                scale: 1.0,
                rot: Quaternion::from_angle_z(Rad(-(time * PI) as f32)),
                disp: Vector3::new(
                    (camera.pos.x + ox) as f32,
                    -(camera.pos.y + oy) as f32,
                    camera.pos.z as f32,
                ),
            });
        }

        {
            let moon = models.get_model(&self.moon).unwrap();
            let camera = renderer.camera.lock();
            moon.matrix[0] = Matrix4::from(Decomposed {
                scale: 1.0,
                rot: Quaternion::from_angle_z(Rad((PI - (time * PI)) as f32)),
                disp: Vector3::new(
                    (camera.pos.x - ox) as f32,
                    -(camera.pos.y - oy) as f32,
                    camera.pos.z as f32,
                ),
            });
        }
    }

    pub fn generate_sun(renderer: Arc<render::Renderer>) -> model::ModelHandle {
        let tex = render::Renderer::get_texture(renderer.get_textures_ref(), "environment/sun");
        renderer.models.lock().create_model(
            model::SUN,
            vec![vec![
                model::Vertex {
                    x: 0.0,
                    y: -SIZE,
                    z: -SIZE,
                    texture_x: 0.0,
                    texture_y: 1.0,
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: SIZE,
                    z: -SIZE,
                    texture_x: 0.0,
                    texture_y: 0.0,
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: -SIZE,
                    z: SIZE,
                    texture_x: 1.0,
                    texture_y: 1.0,
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: SIZE,
                    z: SIZE,
                    texture_x: 1.0,
                    texture_y: 0.0,
                    texture: tex,
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
            ]],
            renderer.clone(),
        )
    }

    pub fn generate_moon(renderer: Arc<render::Renderer>, phase: i32) -> model::ModelHandle {
        let tex =
            render::Renderer::get_texture(renderer.get_textures_ref(), "environment/moon_phases");
        let mpx = (phase % 4) as f64 * (1.0 / 4.0);
        let mpy = (phase / 4) as f64 * (1.0 / 2.0);
        renderer.models.lock().create_model(
            model::SUN,
            vec![vec![
                model::Vertex {
                    x: 0.0,
                    y: -SIZE,
                    z: -SIZE,
                    texture_x: mpx,
                    texture_y: mpy + (1.0 / 2.0),
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: SIZE,
                    z: -SIZE,
                    texture_x: mpx,
                    texture_y: mpy,
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: -SIZE,
                    z: SIZE,
                    texture_x: mpx + (1.0 / 4.0),
                    texture_y: mpy + (1.0 / 2.0),
                    texture: tex.clone(),
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
                model::Vertex {
                    x: 0.0,
                    y: SIZE,
                    z: SIZE,
                    texture_x: mpx + (1.0 / 4.0),
                    texture_y: mpy,
                    texture: tex,
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 0,
                    id: 0,
                },
            ]],
            renderer.clone(),
        )
    }
}
