// Copyright 2016 Matthew Collins
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod atlas;
pub mod glsl;
#[macro_use]
pub mod shaders;
pub mod clouds;
pub mod hud;
pub mod inventory;
pub mod model;
pub mod ui;

// TODO: Fix skin misassignment - happens even if this client joins only one server and stays there
// TODO: But only one person gets a random skin from another person, but if the first person rejoins, their skin gets normal again.
use crate::gl;
use crate::paths;
use crate::resources;
use byteorder::{NativeEndian, WriteBytesExt};
use cgmath::prelude::*;
use image::{GenericImage, GenericImageView, RgbaImage};
use log::error;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use crate::types::hash::FNVHash;
use crate::world::World;
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use image::imageops::FilterType;
use parking_lot::{Mutex, RwLock};
use std::hash::BuildHasherDefault;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicUsize, Ordering};
use std::thread;

const ATLAS_SIZE: usize = 2048;

pub struct Camera {
    pub pos: cgmath::Point3<f64>,
    pub yaw: f64,
    pub pitch: f64,
}

pub struct Renderer {
    resource_version: AtomicUsize,
    pub resources: Arc<RwLock<resources::Manager>>,
    textures: Arc<RwLock<TextureManager>>,
    pub ui: Mutex<ui::UIState>,
    pub models: Arc<Mutex<model::Manager>>,
    pub clouds: Mutex<Option<clouds::Clouds>>,
    texture_data: Mutex<TextureData>,
    chunk_render_data: Mutex<ChunkRenderData>,
    element_buffer_data: Mutex<ElementBufferData>,

    pub camera: Mutex<Camera>,
    perspective_matrix: Mutex<cgmath::Matrix4<f32>>,
    camera_matrix: Mutex<cgmath::Matrix4<f32>>,
    pub frustum: Mutex<collision::Frustum<f32>>,
    pub view_vector: Mutex<cgmath::Vector3<f32>>,

    pub frame_id: AtomicU32,
    pub screen_data: RwLock<ScreenData>,
    pub light_data: Mutex<LightData>,
    skin_exchange_data: Mutex<SkinExchangeData>,
}

struct ChunkRenderData {
    chunk_shader: ChunkShader,
    chunk_shader_alpha: ChunkShaderAlpha,
    trans_shader: Arc<TransShader>,
    trans: Option<TransInfo>,
}

struct ElementBufferData {
    element_buffer: gl::Buffer,
    element_buffer_size: usize,
    element_buffer_type: gl::Type,
}

struct SkinExchangeData {
    skin_request: Sender<String>,
    skin_reply: Receiver<(String, Option<image::DynamicImage>)>,
}

struct TextureData {
    gl_texture: gl::Texture,
    texture_layers: usize,
}

pub struct LightData {
    // Light rendering
    pub light_level: f32,
    pub sky_offset: f32,
}

#[derive(Copy, Clone)]
pub struct ScreenData {
    pub width: u32,
    pub height: u32,
    pub safe_width: u32,
    pub safe_height: u32,
}

impl ScreenData {
    pub fn center(&self) -> (u32, u32) {
        (self.safe_width / 2, self.safe_height / 2)
    }
}

#[derive(Default)]
pub struct ChunkBuffer {
    solid: Option<ChunkRenderInfo>,
    trans: Option<ChunkRenderInfo>,
}

impl ChunkBuffer {
    pub fn new() -> ChunkBuffer {
        Default::default()
    }
}

struct ChunkRenderInfo {
    array: gl::VertexArray,
    buffer: gl::Buffer,
    buffer_size: usize,
    count: usize,
}

init_shader! {
    Program ChunkShader {
        vert = "chunk_vertex",
        frag = "chunk_frag",
        attribute = {
            required position => "aPosition",
            required texture_info => "aTextureInfo",
            required texture_offset => "aTextureOffset",
            required color => "aColor",
            required lighting => "aLighting",
        },
        uniform = {
            required perspective_matrix => "perspectiveMatrix",
            required camera_matrix => "cameraMatrix",
            required offset => "offset",
            required texture => "textures",
            required light_level => "lightLevel",
            required sky_offset => "skyOffset",
        },
    }
}

init_shader! {
    Program ChunkShaderAlpha {
        vert = "chunk_vertex",
        frag = "chunk_frag", #alpha
        attribute = {
            required position => "aPosition",
            required texture_info => "aTextureInfo",
            required texture_offset => "aTextureOffset",
            required color => "aColor",
            required lighting => "aLighting",
        },
        uniform = {
            required perspective_matrix => "perspectiveMatrix",
            required camera_matrix => "cameraMatrix",
            required offset => "offset",
            required texture => "textures",
            required light_level => "lightLevel",
            required sky_offset => "skyOffset",
        },
    }
}

impl Renderer {
    pub fn new(res: Arc<RwLock<resources::Manager>>, shader_version: &str) -> Renderer {
        let version = res.read().version();
        let tex = gl::Texture::new();
        tex.bind(gl::TEXTURE_2D_ARRAY);
        tex.image_3d(
            gl::TEXTURE_2D_ARRAY,
            0,
            ATLAS_SIZE as u32,
            ATLAS_SIZE as u32,
            1,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            &[0; ATLAS_SIZE * ATLAS_SIZE * 4],
        );
        tex.set_parameter(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MAG_FILTER, gl::NEAREST);
        tex.set_parameter(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MIN_FILTER, gl::NEAREST);
        tex.set_parameter(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE);
        tex.set_parameter(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE);

        let (textures, skin_req, skin_reply) = TextureManager::new(res.clone());
        let textures = Arc::new(RwLock::new(textures));

        let mut greg = glsl::Registry::new(shader_version);
        shaders::add_shaders(&mut greg);
        let ui = ui::UIState::new(&greg, textures.clone(), res.clone());

        gl::enable(gl::DEPTH_TEST);
        gl::enable(gl::CULL_FACE_FLAG);
        gl::cull_face(gl::BACK);
        gl::front_face(gl::CLOCK_WISE);

        // Shaders
        let chunk_shader = ChunkShader::new(&greg);
        let chunk_shader_alpha = ChunkShaderAlpha::new(&greg);
        let trans_shader = TransShader::new(&greg);

        // UI
        // Line Drawer
        // Clouds

        gl::blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::depth_func(gl::LESS_OR_EQUAL);

        let clouds = Some(clouds::Clouds::new(&greg, textures.clone()));
        // TODO: setting to disable clouds on native, too, if desired

        Self {
            resource_version: AtomicUsize::from(version),
            models: Arc::new(Mutex::new(model::Manager::new(&greg))),
            clouds: Mutex::new(clouds),
            textures,
            ui: Mutex::new(ui),
            resources: res,
            texture_data: Mutex::new(TextureData {
                gl_texture: tex,
                texture_layers: 1,
            }),
            chunk_render_data: Mutex::new(ChunkRenderData {
                chunk_shader,
                chunk_shader_alpha,
                trans_shader: Arc::new(trans_shader),
                trans: None,
            }),
            camera: Mutex::new(Camera {
                pos: cgmath::Point3::new(0.0, 0.0, 0.0),
                yaw: 0.0,
                pitch: ::std::f64::consts::PI,
            }),
            perspective_matrix: Mutex::new(cgmath::Matrix4::identity()),
            camera_matrix: Mutex::new(cgmath::Matrix4::identity()),
            frustum: Mutex::new(
                collision::Frustum::from_matrix4(cgmath::Matrix4::identity()).unwrap(),
            ),
            view_vector: Mutex::new(cgmath::Vector3::zero()),
            frame_id: AtomicU32::new(1),
            screen_data: RwLock::new(ScreenData {
                width: 0,
                height: 0,
                safe_width: 0,
                safe_height: 0,
            }),
            light_data: Mutex::new(LightData {
                light_level: 0.8,
                sky_offset: 1.0,
            }),
            element_buffer_data: Mutex::new(ElementBufferData {
                element_buffer: gl::Buffer::new(),
                element_buffer_size: 0,
                element_buffer_type: gl::UNSIGNED_BYTE,
            }),
            skin_exchange_data: Mutex::new(SkinExchangeData {
                skin_request: skin_req,
                skin_reply,
            }),
        }
    }

    pub fn reset(&self) {
        self.textures.clone().write().reset();
    }

    pub fn update_camera(&self, width: u32, height: u32) {
        use std::f64::consts::PI as PI64;
        // Not a sane place to put this but it works
        {
            let version = self.resources.read().version();
            if version != self.resource_version.load(Ordering::Acquire) {
                self.resource_version.store(version, Ordering::Release);
                self.textures
                    .write()
                    .update_textures(self.resource_version.load(Ordering::Acquire));

                self.models.lock().rebuild_models(
                    self.resource_version.load(Ordering::Acquire),
                    &self.textures,
                );
            }
        }

        if self.screen_data.read().height != height || self.screen_data.read().width != width {
            self.screen_data.write().width = width;
            self.screen_data.write().height = height;
            self.screen_data.write().safe_width = width;
            self.screen_data.write().safe_height = height;
            gl::viewport(0, 0, width as i32, height as i32);

            let fovy = cgmath::Rad::from(cgmath::Deg(90.0_f32));
            let aspect = (width as f32 / height as f32).max(1.0);

            *self.perspective_matrix.lock() = cgmath::Matrix4::from(cgmath::PerspectiveFov {
                fovy,
                aspect,
                near: 0.1f32,
                far: 500.0f32,
            });

            self.init_trans(width, height);
        }

        let tmp_cam = self.camera.lock();
        *self.view_vector.lock() = cgmath::Vector3::new(
            ((tmp_cam.yaw - PI64 / 2.0).cos() * -tmp_cam.pitch.cos()) as f32,
            (-tmp_cam.pitch.sin()) as f32,
            (-(tmp_cam.yaw - PI64 / 2.0).sin() * -tmp_cam.pitch.cos()) as f32,
        );
        let camera = cgmath::Point3::new(
            -tmp_cam.pos.x as f32,
            -tmp_cam.pos.y as f32,
            tmp_cam.pos.z as f32,
        );
        let view_vec = self.view_vector.lock();
        let camera_matrix = cgmath::Matrix4::look_at(
            camera,
            camera + cgmath::Point3::new(-view_vec.x, -view_vec.y, view_vec.z).to_vec(),
            cgmath::Vector3::new(0.0, -1.0, 0.0),
        );
        drop(view_vec);
        *self.camera_matrix.lock() =
            camera_matrix * cgmath::Matrix4::from_nonuniform_scale(-1.0, 1.0, 1.0);
        *self.frustum.lock() = collision::Frustum::from_matrix4(
            *self.perspective_matrix.lock() * *self.camera_matrix.lock(),
        )
        .unwrap();
    }

    pub fn tick(
        &self,
        world: Option<Arc<World>>,
        delta: f64,
        width: u32,
        height: u32,
        physical_width: u32,
        physical_height: u32,
    ) {
        self.update_textures(delta);

        if world.is_some() {
            if self.chunk_render_data.lock().trans.is_some() {
                let chunk_data = self.chunk_render_data.lock();
                let trans = chunk_data.trans.as_ref().unwrap();
                trans.main.bind();
            }

            gl::active_texture(0);
            self.texture_data
                .lock()
                .gl_texture
                .bind(gl::TEXTURE_2D_ARRAY);

            gl::enable(gl::MULTISAMPLE);

            let time_offset = self.light_data.lock().sky_offset * 0.9;
            gl::clear_color(
                (122.0 / 255.0) * time_offset,
                (165.0 / 255.0) * time_offset,
                (247.0 / 255.0) * time_offset,
                1.0,
            );
            gl::clear(gl::ClearFlags::Color | gl::ClearFlags::Depth);
            // Chunk rendering
            self.chunk_render_data
                .lock()
                .chunk_shader
                .program
                .use_program();

            self.chunk_render_data
                .lock()
                .chunk_shader
                .perspective_matrix
                .set_matrix4(&self.perspective_matrix.lock());
            self.chunk_render_data
                .lock()
                .chunk_shader
                .camera_matrix
                .set_matrix4(&self.camera_matrix.lock());
            self.chunk_render_data
                .lock()
                .chunk_shader
                .texture
                .set_int(0);
            self.chunk_render_data
                .lock()
                .chunk_shader
                .light_level
                .set_float(self.light_data.lock().light_level);
            self.chunk_render_data
                .lock()
                .chunk_shader
                .sky_offset
                .set_float(self.light_data.lock().sky_offset);

            let tmp_world = world.as_ref().unwrap().clone();

            for (pos, info) in tmp_world.get_render_list() {
                if let Some(solid) = info.clone().read().solid.as_ref() {
                    if solid.count > 0 {
                        self.chunk_render_data.lock().chunk_shader.offset.set_int3(
                            pos.0,
                            pos.1 * 4096,
                            pos.2,
                        );
                        solid.array.bind();
                        gl::draw_elements(
                            gl::TRIANGLES,
                            solid.count as i32,
                            self.element_buffer_data.lock().element_buffer_type,
                            0,
                        );
                    }
                }
            }

            // Line rendering
            // Model rendering
            let light_data = self.light_data.lock();
            self.models.lock().draw(
                *self.frustum.lock(), /*&self.frustum*/
                &self.perspective_matrix.lock(),
                &self.camera_matrix.lock(),
                light_data.light_level,
                light_data.sky_offset,
            );
            let tmp_world = world.as_ref().unwrap().clone();

            if let Some(clouds) = &mut *self.clouds.lock() {
                if tmp_world.copy_cloud_heightmap(&mut clouds.heightmap_data) {
                    clouds.dirty = true;
                }
                clouds.draw(
                    &self.camera.lock().pos,
                    &self.perspective_matrix.lock(),
                    &self.camera_matrix.lock(),
                    light_data.light_level,
                    light_data.sky_offset,
                    delta,
                );
            }

            if self.chunk_render_data.lock().trans.is_some() {
                // Trans chunk rendering
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .program
                    .use_program();
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .perspective_matrix
                    .set_matrix4(&self.perspective_matrix.lock());
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .camera_matrix
                    .set_matrix4(&self.camera_matrix.lock());
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .texture
                    .set_int(0);
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .light_level
                    .set_float(light_data.light_level);
                self.chunk_render_data
                    .lock()
                    .chunk_shader_alpha
                    .sky_offset
                    .set_float(light_data.sky_offset);

                // Copy the depth buffer
                let chunk_data = self.chunk_render_data.lock();
                let trans = chunk_data.trans.as_ref().unwrap();
                trans.main.bind_read();
                trans.trans.bind_draw();
            }
        }
        gl::blit_framebuffer(
            0,
            0,
            physical_width as i32,
            physical_height as i32,
            0,
            0,
            physical_width as i32,
            physical_height as i32,
            gl::ClearFlags::Depth,
            gl::NEAREST,
        );

        gl::enable(gl::BLEND);
        gl::depth_mask(false);
        if world.is_some() && self.chunk_render_data.lock().trans.is_some() {
            let chunk_data = self.chunk_render_data.lock();
            let trans = chunk_data.trans.as_ref().unwrap();
            trans.trans.bind();
        }
        gl::clear_color(0.0, 0.0, 0.0, 1.0); // clear color
        gl::clear(gl::ClearFlags::Color);
        gl::clear_buffer(gl::COLOR, 0, &mut [0.0, 0.0, 0.0, 1.0]); // clear color
        gl::clear_buffer(gl::COLOR, 1, &mut [0.0, 0.0, 0.0, 0.0]); // clear color
        gl::blend_func_separate(
            gl::ONE_FACTOR,
            gl::ONE_FACTOR,
            gl::ZERO_FACTOR,
            gl::ONE_MINUS_SRC_ALPHA,
        );

        if world.is_some() {
            let tmp_world = world.as_ref().unwrap().clone();
            for (pos, info) in tmp_world.get_render_list().iter().rev() {
                if let Some(trans) = info.clone().read().trans.as_ref() {
                    if trans.count > 0 {
                        self.chunk_render_data
                            .lock()
                            .chunk_shader_alpha
                            .offset
                            .set_int3(pos.0, pos.1 * 4096, pos.2);
                        trans.array.bind();
                        gl::draw_elements(
                            gl::TRIANGLES,
                            trans.count as i32,
                            self.element_buffer_data.lock().element_buffer_type,
                            0,
                        );
                    }
                }
            }
        }

        gl::check_framebuffer_status();
        gl::unbind_framebuffer();
        gl::disable(gl::DEPTH_TEST);
        gl::clear(gl::ClearFlags::Color);
        gl::disable(gl::BLEND);
        if world.is_some() && self.chunk_render_data.lock().trans.is_some() {
            let mut chunk_data = self.chunk_render_data.lock();
            let shader = chunk_data.trans_shader.clone();
            let trans = chunk_data.trans.as_mut().unwrap();
            trans.draw(&shader);
        }

        gl::enable(gl::DEPTH_TEST);
        gl::depth_mask(true);

        gl::disable(gl::MULTISAMPLE);

        self.ui.lock().tick(width, height);

        gl::check_gl_error();

        self.frame_id
            .fetch_update(Ordering::Release, Ordering::Relaxed, |x| {
                Some(x.wrapping_add(1))
            })
            .unwrap();
    }

    fn ensure_element_buffer(&self, size: usize) {
        if self.element_buffer_data.lock().element_buffer_size < size {
            let (data, ty) = self::generate_element_buffer(size);
            self.element_buffer_data.lock().element_buffer_type = ty;
            self.element_buffer_data
                .lock()
                .element_buffer
                .bind(gl::ELEMENT_ARRAY_BUFFER);
            self.element_buffer_data.lock().element_buffer.set_data(
                gl::ELEMENT_ARRAY_BUFFER,
                &data,
                gl::DYNAMIC_DRAW,
            );
            self.element_buffer_data.lock().element_buffer_size = size;
        }
    }

    pub fn update_chunk_solid(&self, buffer: Arc<RwLock<ChunkBuffer>>, data: &[u8], count: usize) {
        self.ensure_element_buffer(count);
        if count == 0 {
            if buffer.read().solid.is_some() {
                buffer.write().solid = None;
            }
            return;
        }
        let new = buffer.read().solid.is_none();
        if buffer.read().solid.is_none() {
            buffer.write().solid = Some(ChunkRenderInfo {
                array: gl::VertexArray::new(),
                buffer: gl::Buffer::new(),
                buffer_size: 0,
                count: 0,
            });
        }
        let info = buffer;
        let mut info = info.write();
        let info = info.solid.as_mut().unwrap();

        info.array.bind();
        self.chunk_render_data.lock().chunk_shader.position.enable();
        self.chunk_render_data
            .lock()
            .chunk_shader
            .texture_info
            .enable();
        self.chunk_render_data
            .lock()
            .chunk_shader
            .texture_offset
            .enable();
        self.chunk_render_data.lock().chunk_shader.color.enable();
        self.chunk_render_data.lock().chunk_shader.lighting.enable();

        self.element_buffer_data
            .lock()
            .element_buffer
            .bind(gl::ELEMENT_ARRAY_BUFFER);

        info.buffer.bind(gl::ARRAY_BUFFER);
        if new || info.buffer_size < data.len() {
            info.buffer_size = data.len();
            info.buffer
                .set_data(gl::ARRAY_BUFFER, data, gl::DYNAMIC_DRAW);
        } else {
            info.buffer.re_set_data(gl::ARRAY_BUFFER, data);
        }

        self.chunk_render_data
            .lock()
            .chunk_shader
            .position
            .vertex_pointer(3, gl::FLOAT, false, 40, 0);
        self.chunk_render_data
            .lock()
            .chunk_shader
            .texture_info
            .vertex_pointer(4, gl::UNSIGNED_SHORT, false, 40, 12);
        self.chunk_render_data
            .lock()
            .chunk_shader
            .texture_offset
            .vertex_pointer(3, gl::SHORT, false, 40, 20);
        self.chunk_render_data
            .lock()
            .chunk_shader
            .color
            .vertex_pointer(3, gl::UNSIGNED_BYTE, true, 40, 28);
        self.chunk_render_data
            .lock()
            .chunk_shader
            .lighting
            .vertex_pointer(2, gl::UNSIGNED_SHORT, false, 40, 32);

        info.count = count;
    }

    pub fn update_chunk_trans(&self, buffer: Arc<RwLock<ChunkBuffer>>, data: &[u8], count: usize) {
        self.ensure_element_buffer(count);
        if count == 0 {
            if buffer.read().trans.is_some() {
                buffer.write().trans = None;
            }
            return;
        }
        let new = buffer.read().trans.is_none();
        if buffer.read().trans.is_none() {
            buffer.write().trans = Some(ChunkRenderInfo {
                array: gl::VertexArray::new(),
                buffer: gl::Buffer::new(),
                buffer_size: 0,
                count: 0,
            });
        }
        let info = buffer;
        let mut info = info.write();
        let info = info.trans.as_mut().unwrap();

        info.array.bind();
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .position
            .enable();
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .texture_info
            .enable();
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .texture_offset
            .enable();
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .color
            .enable();
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .lighting
            .enable();

        self.element_buffer_data
            .lock()
            .element_buffer
            .bind(gl::ELEMENT_ARRAY_BUFFER);

        info.buffer.bind(gl::ARRAY_BUFFER);
        if new || info.buffer_size < data.len() {
            info.buffer_size = data.len();
            info.buffer
                .set_data(gl::ARRAY_BUFFER, data, gl::DYNAMIC_DRAW);
        } else {
            info.buffer.re_set_data(gl::ARRAY_BUFFER, data);
        }

        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .position
            .vertex_pointer(3, gl::FLOAT, false, 40, 0);
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .texture_info
            .vertex_pointer(4, gl::UNSIGNED_SHORT, false, 40, 12);
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .texture_offset
            .vertex_pointer(3, gl::SHORT, false, 40, 20);
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .color
            .vertex_pointer(3, gl::UNSIGNED_BYTE, true, 40, 28);
        self.chunk_render_data
            .lock()
            .chunk_shader_alpha
            .lighting
            .vertex_pointer(2, gl::UNSIGNED_SHORT, false, 40, 32);

        info.count = count;
    }

    #[allow(clippy::uninit_vec)]
    fn do_pending_textures(&self) {
        let len = {
            let tex = self.textures.read();
            // Rebuild the texture if it needs resizing
            if self.texture_data.lock().texture_layers != tex.atlases.len() {
                let len = ATLAS_SIZE * ATLAS_SIZE * 4 * tex.atlases.len();
                let mut data = Vec::with_capacity(len);
                // We are creating uninitialized values here, but as the renderer should be replaced with wgpu-mc soon-ish this isn't worth fixing.
                unsafe {
                    data.set_len(len);
                }
                self.texture_data.lock().gl_texture.get_pixels(
                    gl::TEXTURE_2D_ARRAY,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    &mut data[..],
                );
                self.texture_data.lock().gl_texture.image_3d(
                    gl::TEXTURE_2D_ARRAY,
                    0,
                    ATLAS_SIZE as u32,
                    ATLAS_SIZE as u32,
                    tex.atlases.len() as u32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    &data[..],
                );
                self.texture_data.lock().texture_layers = tex.atlases.len();
            }
            tex.pending_uploads.len()
        };
        if len > 0 {
            // Upload pending changes
            let mut tex = self.textures.write();
            for upload in &tex.pending_uploads {
                let atlas = upload.0;
                let rect = upload.1;
                let img = &upload.2;
                self.texture_data.lock().gl_texture.sub_image_3d(
                    gl::TEXTURE_2D_ARRAY,
                    0,
                    rect.x as u32,
                    rect.y as u32,
                    atlas as u32,
                    rect.width as u32,
                    rect.height as u32,
                    1,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    &img[..],
                );
            }
            tex.pending_uploads.clear();
        }
    }

    fn update_textures(&self, delta: f64) {
        {
            let mut tex = self.textures.write();
            while let Ok((hash, img)) = self.skin_exchange_data.lock().skin_reply.try_recv() {
                if let Some(img) = img {
                    tex.update_skin(hash, img);
                }
            }
            let mut old_skins = vec![];
            for (skin, refcount) in &tex.skins {
                if refcount.load(Ordering::Relaxed) <= 0 {
                    old_skins.push(skin.clone());
                }
            }
            for skin in old_skins {
                tex.skins.remove(&skin);
                tex.remove_dynamic(&format!("skin-{}", skin));
            }
        }
        self.texture_data
            .lock()
            .gl_texture
            .bind(gl::TEXTURE_2D_ARRAY);
        self.do_pending_textures();

        for ani in &mut self.textures.write().animated_textures {
            if ani.remaining_time <= 0.0 {
                ani.current_frame = (ani.current_frame + 1) % ani.frames.len();
                ani.remaining_time += ani.frames[ani.current_frame].time as f64;
                let offset =
                    ani.texture.width * ani.texture.width * ani.frames[ani.current_frame].index * 4;
                let offset2 = offset + ani.texture.width * ani.texture.width * 4;
                self.texture_data.lock().gl_texture.sub_image_3d(
                    gl::TEXTURE_2D_ARRAY,
                    0,
                    ani.texture.get_x() as u32,
                    ani.texture.get_y() as u32,
                    ani.texture.atlas as u32,
                    ani.texture.get_width() as u32,
                    ani.texture.get_height() as u32,
                    1,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    &ani.data[offset..offset2],
                );
            } else {
                ani.remaining_time -= delta / 3.0;
            }
        }
    }

    fn init_trans(&self, width: u32, height: u32) {
        self.chunk_render_data.lock().trans = None;
        let mut chunk_render_data = self.chunk_render_data.lock();
        chunk_render_data.trans = Some(TransInfo::new(
            width,
            height,
            &chunk_render_data.chunk_shader_alpha,
            &chunk_render_data.trans_shader,
        ));
    }

    pub fn get_textures(&self) -> Arc<RwLock<TextureManager>> {
        self.textures.clone()
    }

    pub fn get_textures_ref(&self) -> &RwLock<TextureManager> {
        &self.textures
    }

    pub fn check_texture(&self, tex: Texture) -> Texture {
        if tex.version == self.resource_version.load(Ordering::Acquire) {
            tex
        } else {
            let mut new = Renderer::get_texture(&self.textures, &tex.name);
            new.rel_x = tex.rel_x;
            new.rel_y = tex.rel_y;
            new.rel_width = tex.rel_width;
            new.rel_height = tex.rel_height;
            new.is_rel = tex.is_rel;
            new
        }
    }

    pub fn get_texture(textures: &RwLock<TextureManager>, name: &str) -> Texture {
        let tex = { textures.read().get_texture(name) };
        match tex {
            Some(val) => val,
            None => {
                let mut t = textures.write();
                // Make sure it hasn't already been loaded since we switched
                // locks.
                if let Some(val) = t.get_texture(name) {
                    val
                } else {
                    t.load_texture(name);
                    t.get_texture(name).unwrap()
                }
            }
        }
    }

    pub fn get_texture_optional(textures: &RwLock<TextureManager>, name: &str) -> Option<Texture> {
        let tex = { textures.read().get_texture(name) };
        match tex {
            Some(val) => Some(val),
            None => {
                let mut t = textures.write();
                // Make sure it hasn't already been loaded since we switched
                // locks.
                if let Some(val) = t.get_texture(name) {
                    Some(val)
                } else {
                    t.load_texture(name);
                    t.get_texture(name)
                }
            }
        }
    }

    pub fn get_skin(&self, textures: &RwLock<TextureManager>, url: &str) -> Texture {
        let tex = { textures.read().get_skin(url) };
        match tex {
            Some(val) => val,
            None => {
                let mut t = textures.write();
                // Make sure it hasn't already been loaded since we switched
                // locks.
                if let Some(val) = t.get_skin(url) {
                    val
                } else {
                    t.load_skin(self, url);
                    t.get_skin(url).unwrap()
                }
            }
        }
    }
}

struct TransInfo {
    main: gl::Framebuffer,
    fb_color: gl::Texture,
    _fb_depth: gl::Texture,
    trans: gl::Framebuffer,
    accum: gl::Texture,
    revealage: gl::Texture,
    _depth: gl::Texture,

    array: gl::VertexArray,
    _buffer: gl::Buffer,
}

init_shader! {
    Program TransShader {
        vert = "trans_vertex",
        frag = "trans_frag",
        attribute = {
            required position => "aPosition",
        },
        uniform = {
            required accum => "taccum",
            required revealage => "trevealage",
            required color => "tcolor",
        },
    }
}

impl TransInfo {
    pub fn new(
        width: u32,
        height: u32,
        chunk_shader: &ChunkShaderAlpha,
        shader: &TransShader,
    ) -> TransInfo {
        let trans = gl::Framebuffer::new();
        trans.bind();

        let accum = gl::Texture::new();
        accum.bind(gl::TEXTURE_2D);
        accum.image_2d_ex(
            gl::TEXTURE_2D,
            0,
            width,
            height,
            gl::RGBA16F,
            gl::RGBA,
            gl::FLOAT,
            None,
        );
        accum.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR);
        accum.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR);
        trans.texture_2d(gl::COLOR_ATTACHMENT_0, gl::TEXTURE_2D, &accum, 0);

        let revealage = gl::Texture::new();
        revealage.bind(gl::TEXTURE_2D);
        revealage.image_2d_ex(
            gl::TEXTURE_2D,
            0,
            width,
            height,
            gl::R16F,
            gl::RED,
            gl::FLOAT,
            None,
        );
        revealage.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR);
        revealage.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR);
        trans.texture_2d(gl::COLOR_ATTACHMENT_1, gl::TEXTURE_2D, &revealage, 0);

        let trans_depth = gl::Texture::new();
        trans_depth.bind(gl::TEXTURE_2D);
        trans_depth.image_2d_ex(
            gl::TEXTURE_2D,
            0,
            width,
            height,
            gl::DEPTH_COMPONENT24,
            gl::DEPTH_COMPONENT,
            gl::UNSIGNED_INT,
            None,
        );
        trans_depth.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR);
        trans_depth.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR);
        trans.texture_2d(gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, &trans_depth, 0);

        chunk_shader.program.use_program();
        // bound with layout(location=)
        gl::bind_frag_data_location(&chunk_shader.program, 0, "accum");
        gl::bind_frag_data_location(&chunk_shader.program, 1, "revealage");
        gl::check_framebuffer_status();
        gl::draw_buffers(&[gl::COLOR_ATTACHMENT_0, gl::COLOR_ATTACHMENT_1]);

        let main = gl::Framebuffer::new();
        main.bind();

        // TODO: support rendering to a multisample renderbuffer for MSAA, using glRenderbufferStorageMultisample
        // https://github.com/iceiix/stevenarella/pull/442
        let fb_color = gl::Texture::new();
        fb_color.bind(gl::TEXTURE_2D);
        fb_color.image_2d(
            gl::TEXTURE_2D,
            0,
            width,
            height,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            None,
        );
        fb_color.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR);
        fb_color.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR);

        main.texture_2d(gl::COLOR_ATTACHMENT_0, gl::TEXTURE_2D, &fb_color, 0);
        let fb_depth = gl::Texture::new();
        fb_depth.bind(gl::TEXTURE_2D);
        fb_depth.image_2d_ex(
            gl::TEXTURE_2D,
            0,
            width,
            height,
            gl::DEPTH_COMPONENT24,
            gl::DEPTH_COMPONENT,
            gl::UNSIGNED_INT,
            None,
        );
        fb_depth.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR);
        fb_depth.set_parameter(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR);

        main.texture_2d(gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, &fb_depth, 0);
        gl::check_framebuffer_status();

        gl::unbind_framebuffer();

        shader.program.use_program();
        let array = gl::VertexArray::new();
        array.bind();
        let buffer = gl::Buffer::new();
        buffer.bind(gl::ARRAY_BUFFER);

        let mut data = vec![];
        for f in [
            -1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0,
        ]
        .iter()
        {
            data.write_f32::<NativeEndian>(*f).unwrap();
        }
        buffer.set_data(gl::ARRAY_BUFFER, &data, gl::STATIC_DRAW);

        shader.position.enable();
        shader.position.vertex_pointer(2, gl::FLOAT, false, 8, 0);

        TransInfo {
            main,
            fb_color,
            _fb_depth: fb_depth,
            trans,
            accum,
            revealage,
            _depth: trans_depth,

            array,
            _buffer: buffer,
        }
    }

    fn draw(&mut self, shader: &TransShader) {
        gl::active_texture(0);
        self.accum.bind(gl::TEXTURE_2D);
        gl::active_texture(1);
        self.revealage.bind(gl::TEXTURE_2D);
        gl::active_texture(2);
        self.fb_color.bind(gl::TEXTURE_2D);

        shader.program.use_program();
        shader.accum.set_int(0);
        shader.revealage.set_int(1);
        shader.color.set_int(2);
        self.array.bind();
        gl::draw_arrays(gl::TRIANGLES, 0, 6);
    }
}

pub struct TextureManager {
    textures: HashMap<String, Texture, BuildHasherDefault<FNVHash>>,
    version: usize,
    resources: Arc<RwLock<resources::Manager>>,
    atlases: Vec<atlas::Atlas>,

    animated_textures: Vec<AnimatedTexture>,
    pending_uploads: Vec<(i32, atlas::Rect, Vec<u8>)>,

    dynamic_textures: HashMap<String, (Texture, image::DynamicImage), BuildHasherDefault<FNVHash>>,
    free_dynamics: Vec<Texture>,

    skins: HashMap<String, AtomicIsize, BuildHasherDefault<FNVHash>>,

    _skin_thread: Option<thread::JoinHandle<()>>,
}

impl TextureManager {
    #[allow(clippy::let_and_return)]
    #[allow(clippy::type_complexity)]
    fn new(
        res: Arc<RwLock<resources::Manager>>,
    ) -> (
        TextureManager,
        Sender<String>,
        Receiver<(String, Option<image::DynamicImage>)>,
    ) {
        let (tx, rx) = unbounded();
        let (stx, srx) = unbounded();

        let skin_thread = Some(thread::spawn(|| Self::process_skins(srx, tx)));

        let mut tm = TextureManager {
            textures: HashMap::with_hasher(BuildHasherDefault::default()),
            version: {
                // TODO: fix borrow and remove clippy::let_and_return above
                let ver = res.read().version();
                ver
            },
            resources: res,
            atlases: Vec::new(),
            animated_textures: Vec::new(),
            pending_uploads: Vec::new(),

            dynamic_textures: HashMap::with_hasher(BuildHasherDefault::default()),
            free_dynamics: Vec::new(),
            skins: HashMap::with_hasher(BuildHasherDefault::default()),

            _skin_thread: skin_thread,
        };
        tm.add_defaults();
        (tm, stx, rx)
    }

    pub fn reset(&mut self) {
        self.skins
            .iter_mut()
            .for_each(|skin| skin.1.store(0, Ordering::Relaxed));
        /*for skin in self.skins.drain() {
            self.unload_skin(skin.0.as_str())
        }*/
    }

    fn add_defaults(&mut self) {
        self.put_texture(
            "leafish",
            "missing_texture",
            2,
            2,
            vec![
                0, 0, 0, 255, 255, 0, 255, 255, 255, 0, 255, 255, 0, 0, 0, 255,
            ],
        );
        self.put_texture("leafish", "solid", 1, 1, vec![255, 255, 255, 255]);
    }

    fn process_skins(recv: Receiver<String>, reply: Sender<(String, Option<image::DynamicImage>)>) {
        let client = reqwest::blocking::Client::new();
        loop {
            let hash = match recv.recv() {
                Ok(val) => val,
                Err(_) => return, // Most likely shutting down
            };
            match Self::obtain_skin(&client, &hash) {
                Ok(img) => {
                    let _ = reply.send((hash, Some(img)));
                }
                Err(err) => {
                    error!("Failed to get skin {:?}: {}", hash, err);
                    let _ = reply.send((hash, None));
                }
            }
        }
    }

    fn obtain_skin(
        client: &::reqwest::blocking::Client,
        hash: &str,
    ) -> Result<image::DynamicImage, ::std::io::Error> {
        use std::fs;
        use std::io::Read;
        use std::io::{Error, ErrorKind};
        use std::path::Path;
        let path = paths::get_cache_dir().join(format!("skin-cache/{}/{}.png", &hash[..2], hash));
        let cache_path = Path::new(&path);
        fs::create_dir_all(cache_path.parent().unwrap())?;
        let mut buf = vec![];
        if fs::metadata(cache_path).is_ok() {
            // We have a cached image
            let mut file = fs::File::open(cache_path)?;
            file.read_to_end(&mut buf)?;
        } else {
            // Need to download it
            let url = &format!("http://textures.minecraft.net/texture/{}", hash);
            let mut res = match client.get(url).send() {
                Ok(val) => val,
                Err(err) => {
                    return Err(Error::new(ErrorKind::ConnectionAborted, err));
                }
            };

            match res.read_to_end(&mut buf) {
                Ok(_) => {}
                Err(err) => {
                    // TODO: different error for failure to read?
                    return Err(Error::new(ErrorKind::InvalidData, err));
                }
            }

            // Save to cache
            let mut file = fs::File::create(cache_path)?;
            file.write_all(&buf)?;
        }
        let mut img = match image::load_from_memory(&buf) {
            Ok(val) => val,
            Err(err) => {
                return Err(Error::new(ErrorKind::InvalidData, err));
            }
        };
        let (_, height) = img.dimensions();
        if height == 32 {
            // Needs changing to the new format
            let mut new = image::DynamicImage::new_rgba8(64, 64);
            new.copy_from(&img, 0, 0)
                .expect("Invalid png image in skin");
            for xx in 0..4 {
                for yy in 0..16 {
                    for section in 0..4 {
                        let os = match section {
                            0 => 2,
                            1 => 1,
                            2 => 0,
                            3 => 3,
                            _ => unreachable!(),
                        };
                        new.put_pixel(
                            16 + (3 - xx) + section * 4,
                            48 + yy,
                            img.get_pixel(xx + os * 4, 16 + yy),
                        );
                        new.put_pixel(
                            32 + (3 - xx) + section * 4,
                            48 + yy,
                            img.get_pixel(xx + 40 + os * 4, 16 + yy),
                        );
                    }
                }
            }
            img = new;
        }
        // Block transparent pixels in blacklisted areas
        let blacklist = [
            // X, Y, W, H
            (0, 0, 32, 16),
            (16, 16, 24, 16),
            (0, 16, 16, 16),
            (16, 48, 16, 16),
            (32, 48, 16, 16),
            (40, 16, 16, 16),
        ];
        for bl in blacklist.iter() {
            for x in bl.0..(bl.0 + bl.2) {
                for y in bl.1..(bl.1 + bl.3) {
                    let mut col = img.get_pixel(x, y);
                    col.0[3] = 255;
                    img.put_pixel(x, y, col);
                }
            }
        }
        Ok(img)
    }

    fn update_textures(&mut self, version: usize) {
        self.pending_uploads.clear();
        self.atlases.clear();
        self.animated_textures.clear();
        self.version = version;
        let map = self.textures.clone();
        self.textures.clear();

        self.free_dynamics.clear();

        self.add_defaults();

        for name in map.keys() {
            if let Some(n) = name.strip_prefix("leafish-dynamic:") {
                let (width, height, data) = {
                    let dynamic_texture = match self.dynamic_textures.get(n) {
                        Some(val) => val,
                        None => continue,
                    };
                    let img = &dynamic_texture.1;
                    let (width, height) = img.dimensions();
                    (width, height, img.to_rgba8().into_vec())
                };
                let new_tex = self.put_texture("leafish-dynamic", n, width, height, data);
                self.dynamic_textures.get_mut(n).unwrap().0 = new_tex;
            } else if !self.textures.contains_key(name) {
                self.load_texture(name);
            }
        }
    }

    fn get_skin(&self, url: &str) -> Option<Texture> {
        let hash = &url["http://textures.minecraft.net/texture/".len()..];
        if let Some(skin) = self.skins.get(hash) {
            skin.fetch_add(1, Ordering::Relaxed);
        }
        self.get_texture(&format!("leafish-dynamic:skin-{}", hash))
    }

    pub fn release_skin(&self, url: &str) {
        let hash = &url["http://textures.minecraft.net/texture/".len()..];
        if let Some(skin) = self.skins.get(hash) {
            skin.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn load_skin(&mut self, renderer: &Renderer, url: &str) {
        let hash = &url["http://textures.minecraft.net/texture/".len()..];
        let res = self.resources.clone();
        // TODO: This shouldn't be hardcoded to steve but instead
        // have a way to select alex as a default.
        let img = if let Some(mut val) = res.read().open("minecraft", "textures/entity/steve.png") {
            let mut data = Vec::new();
            val.read_to_end(&mut data).unwrap();
            image::load_from_memory(&data).unwrap()
        } else {
            image::DynamicImage::new_rgba8(64, 64)
        };
        self.put_dynamic(&format!("skin-{}", hash), img);
        self.skins.insert(hash.to_owned(), AtomicIsize::new(0));
        renderer
            .skin_exchange_data
            .lock()
            .skin_request
            .send(hash.to_owned())
            .unwrap();
    }

    // TODO: make use of "unload_skin"
    #[allow(dead_code)]
    fn unload_skin(&mut self, url: &str) {
        self.remove_dynamic(&format!("skin-{}", url));
        self.skins.remove(url);
    }

    fn update_skin(&mut self, hash: String, img: image::DynamicImage) {
        if !self.skins.contains_key(&hash) {
            return;
        }
        let name = format!("leafish-dynamic:skin-{}", hash);
        let tex = self.get_texture(&name).unwrap();
        let rect = atlas::Rect {
            x: tex.x,
            y: tex.y,
            width: tex.width,
            height: tex.height,
        };

        self.pending_uploads
            .push((tex.atlas, rect, img.to_rgba8().into_vec()));
        self.dynamic_textures
            .get_mut(&format!("skin-{}", hash))
            .unwrap()
            .1 = img;
    }

    fn get_texture(&self, name: &str) -> Option<Texture> {
        if name.starts_with('#') {
            let name = if let Some(name) = name.strip_prefix('#') {
                name
            } else {
                name
            };
            self.textures.get(&format!("global:{}", name)).cloned()
        } else if name.find(':').is_some() {
            let name = if let Some(name) = name.strip_prefix('#') {
                name
            } else {
                name
            };
            self.textures.get(name).cloned()
        } else {
            self.textures.get(&format!("minecraft:{}", name)).cloned()
        }
    }

    fn load_texture(&mut self, name: &str) {
        let (plugin, name) = if name.starts_with('#') {
            let name = if let Some(name) = name.strip_prefix('#') {
                name
            } else {
                name
            };
            ("global", name)
        } else if let Some(mut pos) = name.find(':') {
            let name = if let Some(name) = name.strip_prefix('#') {
                pos -= 1;
                name
            } else {
                name
            };
            (&name[..pos], &name[pos + 1..])
        } else {
            ("minecraft", name)
        };
        let path = if plugin != "global" {
            format!("textures/{}.png", name)
        } else {
            name.to_string()
        };
        let res = self.resources.clone();
        if let Some(mut val) = res.read().open(plugin, &path) {
            let mut data = Vec::new();
            val.read_to_end(&mut data).unwrap();
            if let Ok(img) = image::load_from_memory(&data) {
                let (width, height) = img.dimensions();
                // Might be animated
                if (name.starts_with("block/") || name.starts_with("item/")) && width != height {
                    let id = img.to_rgba8().into_vec();
                    let frame = id[..(width * width * 4) as usize].to_owned();
                    if let Some(mut ani) = self.load_animation(plugin, name, &img, id) {
                        ani.texture = self.put_texture(plugin, name, width, width, frame);
                        self.animated_textures.push(ani);
                        return;
                    }
                }
                self.put_texture(plugin, name, width, height, img.to_rgba8().into_vec());
                return;
            }
        }
        self.insert_texture_dummy(plugin, name);
    }

    fn load_animation(
        &mut self,
        plugin: &str,
        name: &str,
        img: &image::DynamicImage,
        data: Vec<u8>,
    ) -> Option<AnimatedTexture> {
        let path = format!("textures/{}.png.mcmeta", name);
        let res = self.resources.clone();
        if let Some(val) = res.read().open(plugin, &path) {
            let meta: serde_json::Value = serde_json::from_reader(val).unwrap();
            let animation = meta.get("animation").unwrap();
            let frame_time = animation
                .get("frametime")
                .and_then(|v| v.as_i64())
                .unwrap_or(1);
            let interpolate = animation
                .get("interpolate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let frames = if let Some(frames) = animation.get("frames").and_then(|v| v.as_array()) {
                let mut out = Vec::with_capacity(frames.len());
                for frame in frames {
                    if let Some(index) = frame.as_i64() {
                        out.push(AnimationFrame {
                            index: index as usize,
                            time: frame_time,
                        })
                    } else {
                        out.push(AnimationFrame {
                            index: frame.get("index").unwrap().as_i64().unwrap() as usize,
                            time: frame_time * frame.get("frameTime").unwrap().as_i64().unwrap(),
                        })
                    }
                }
                out
            } else {
                let (width, height) = img.dimensions();
                let count = height / width;
                let mut frames = Vec::with_capacity(count as usize);
                for i in 0..count {
                    frames.push(AnimationFrame {
                        index: i as usize,
                        time: frame_time,
                    })
                }
                frames
            };

            return Some(AnimatedTexture {
                frames,
                data,
                interpolate,
                current_frame: 0,
                remaining_time: 0.0,
                texture: self.get_texture("leafish:missing_texture").unwrap(),
            });
        }
        None
    }

    fn put_texture(
        &mut self,
        plugin: &str,
        name: &str,
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> Texture {
        let (image, width, height) = if width > ATLAS_SIZE as u32 || height > ATLAS_SIZE as u32 {
            let image = RgbaImage::from_raw(width, height, data).unwrap();
            let scale = width.max(height) as f64 / ATLAS_SIZE as f64;
            let width = (width as f64 / scale) as u32;
            let height = (height as f64 / scale) as u32;
            (
                image::imageops::resize(&image, width, height, FilterType::Nearest)
                    .as_raw()
                    .clone(),
                width,
                height,
            )
        } else {
            (data, width, height)
        };
        let (atlas, rect) = self.find_free(width as usize, height as usize);
        self.pending_uploads.push((atlas, rect, image));

        let mut full_name = String::new();
        full_name.push_str(plugin);
        full_name.push(':');
        full_name.push_str(name);

        let tex = Texture {
            name: full_name.clone(),
            version: self.version,
            atlas,
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            rel_x: 0.0,
            rel_y: 0.0,
            rel_width: 1.0,
            rel_height: 1.0,
            is_rel: false,
            dummy: false,
        };
        self.textures.insert(full_name, tex.clone());
        tex
    }

    fn find_free(&mut self, width: usize, height: usize) -> (i32, atlas::Rect) {
        let mut index = 0;
        for atlas in &mut self.atlases {
            if let Some(rect) = atlas.add(width, height) {
                return (index, rect);
            }
            index += 1;
        }
        let mut atlas = atlas::Atlas::new(ATLAS_SIZE, ATLAS_SIZE);
        let rect = atlas.add(width, height);
        self.atlases.push(atlas);
        (index, rect.unwrap())
    }

    fn insert_texture_dummy(&mut self, plugin: &str, name: &str) -> Texture {
        let missing = self.get_texture("leafish:missing_texture").unwrap();

        let mut full_name = String::new();
        full_name.push_str(plugin);
        full_name.push(':');
        full_name.push_str(name);

        let t = Texture {
            name: full_name.to_owned(),
            version: self.version,
            atlas: missing.atlas,
            x: missing.x,
            y: missing.y,
            width: missing.width,
            height: missing.height,
            rel_x: 0.0,
            rel_y: 0.0,
            rel_width: 1.0,
            rel_height: 1.0,
            is_rel: false,
            dummy: true,
        };
        self.textures.insert(full_name, t.clone());
        t
    }

    pub fn put_dynamic(&mut self, name: &str, img: image::DynamicImage) -> Texture {
        use std::mem;
        let (width, height) = img.dimensions();
        let (width, height) = (width as usize, height as usize);
        let mut rect_pos = None;
        for (i, r) in self.free_dynamics.iter().enumerate() {
            if r.width == width && r.height == height {
                rect_pos = Some(i);
                break;
            } else if r.width >= width && r.height >= height {
                rect_pos = Some(i);
            }
        }
        let data = img.to_rgba8().into_vec();

        if let Some(rect_pos) = rect_pos {
            let mut tex = self.free_dynamics.remove(rect_pos);
            let rect = atlas::Rect {
                x: tex.x,
                y: tex.y,
                width,
                height,
            };
            self.pending_uploads.push((tex.atlas, rect, data));
            let mut t = tex.relative(
                0.0,
                0.0,
                (width as f32) / (tex.width as f32),
                (height as f32) / (tex.height as f32),
            );
            let old_name = mem::replace(&mut tex.name, format!("leafish-dynamic:{}", name));
            self.dynamic_textures.insert(name.to_owned(), (tex, img));
            // We need to rename the texture itself so that get_texture calls
            // work with the new name
            let mut old = self.textures.remove(&old_name).unwrap();
            old.name = format!("leafish-dynamic:{}", name);
            t.name.clone_from(&old.name);
            self.textures
                .insert(format!("leafish-dynamic:{}", name), old);
            t
        } else {
            let tex = self.put_texture("leafish-dynamic", name, width as u32, height as u32, data);
            self.dynamic_textures
                .insert(name.to_owned(), (tex.clone(), img));
            tex
        }
    }

    pub fn remove_dynamic(&mut self, name: &str) {
        let desc = self.dynamic_textures.remove(name).unwrap();
        self.free_dynamics.push(desc.0);
    }
}

#[allow(dead_code)]
struct AnimatedTexture {
    frames: Vec<AnimationFrame>,
    data: Vec<u8>,
    interpolate: bool,
    current_frame: usize,
    remaining_time: f64,
    texture: Texture,
}

struct AnimationFrame {
    index: usize,
    time: i64,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    version: usize,
    pub atlas: i32,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    is_rel: bool, // Save some cycles for non-relative textures
    rel_x: f32,
    rel_y: f32,
    rel_width: f32,
    rel_height: f32,
    dummy: bool,
}

impl Texture {
    pub fn get_x(&self) -> usize {
        if self.is_rel {
            self.x + ((self.width as f32) * self.rel_x) as usize
        } else {
            self.x
        }
    }

    pub fn get_y(&self) -> usize {
        if self.is_rel {
            self.y + ((self.height as f32) * self.rel_y) as usize
        } else {
            self.y
        }
    }

    pub fn get_width(&self) -> usize {
        if self.is_rel {
            ((self.width as f32) * self.rel_width) as usize
        } else {
            self.width
        }
    }

    pub fn get_height(&self) -> usize {
        if self.is_rel {
            ((self.height as f32) * self.rel_height) as usize
        } else {
            self.height
        }
    }

    pub fn relative(&self, x: f32, y: f32, width: f32, height: f32) -> Texture {
        Texture {
            name: self.name.clone(),
            version: self.version,
            x: self.x,
            y: self.y,
            atlas: self.atlas,
            width: self.width,
            height: self.height,
            is_rel: true,
            rel_x: self.rel_x + x * self.rel_width,
            rel_y: self.rel_y + y * self.rel_height,
            rel_width: width * self.rel_width,
            rel_height: height * self.rel_height,
            dummy: self.dummy,
        }
    }
}

#[allow(unused_must_use)]
pub fn generate_element_buffer(size: usize) -> (Vec<u8>, gl::Type) {
    let mut ty = gl::UNSIGNED_SHORT;
    let mut data = if (size / 6) * 4 * 3 >= u16::MAX as usize {
        ty = gl::UNSIGNED_INT;
        Vec::with_capacity(size * 4)
    } else {
        Vec::with_capacity(size * 2)
    };
    for i in 0..size / 6 {
        for val in &[0, 1, 2, 2, 1, 3] {
            if ty == gl::UNSIGNED_INT {
                data.write_u32::<NativeEndian>((i as u32) * 4 + val);
            } else {
                data.write_u16::<NativeEndian>((i as u16) * 4 + (*val as u16));
            }
        }
    }

    (data, ty)
}
