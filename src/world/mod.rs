// Copyright 2015 Matthew Collins
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

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::BuildHasherDefault;
use std::io::{Cursor, Read};
use std::sync::{Arc, RwLock};

use cgmath::prelude::*;
use flate2::read::ZlibDecoder;
pub use leafish_blocks as block;

use crate::chunk_builder;
use crate::ecs;
use crate::entity::block_entity;
use crate::format;
use crate::protocol;
use crate::render;
use crate::shared::{Direction, Position};
use crate::types::{bit, nibble};
use crate::types::hash::FNVHash;
use byteorder::ReadBytesExt;
use instant::Instant;

pub mod biome;
mod storage;

use crate::render::Renderer;
use collision::Frustum;
use crate::chunk_builder::CullInfo;

#[derive(Default)]
pub struct World {
    pub chunks: Arc<RwLock<HashMap<CPos, Chunk, BuildHasherDefault<FNVHash>>>>,
    pub lighting_cache: Arc<RwLock<HashMap<CPos, LightData>>>,

    pub render_list: Arc<RwLock<Vec<(i32, i32, i32)>>>,

    pub(crate) light_updates: Arc<RwLock<VecDeque<LightUpdate>>>,

    block_entity_actions: Arc<RwLock<VecDeque<BlockEntityAction>>>,

    protocol_version: i32,
    pub modded_block_ids: Arc<RwLock<HashMap<usize, String>>>,
    pub id_map: Arc<block::VanillaIDMap>,
}

pub struct LightData {

    pub arrays: Cursor<Vec<u8>>,
    pub block_light_mask: i32,
    pub sky_light_mask: i32,

}

#[derive(Clone, Debug)]
pub enum BlockEntityAction {
    Create(Position),
    Remove(Position),
    UpdateSignText(
        Box<(
            Position,
            format::Component,
            format::Component,
            format::Component,
            format::Component,
        )>,
    ),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LightType {
    Block,
    Sky,
}

impl LightType {
    fn get_light(self, world: &World, pos: Position) -> u8 {
        match self {
            LightType::Block => world.get_block_light(pos),
            LightType::Sky => world.get_sky_light(pos),
        }
    }
    fn set_light(self, world: &World, pos: Position, light: u8) {
        match self {
            LightType::Block => world.set_block_light(pos, light),
            LightType::Sky => world.set_sky_light(pos, light),
        }
    }
}

pub struct LightUpdate {
    ty: LightType,
    pos: Position,
}

impl World {
    pub fn new(protocol_version: i32) -> World {
        let id_map = Arc::new(block::VanillaIDMap::new(protocol_version));
        World {
            protocol_version,
            id_map,
            ..Default::default()
        }
    }

    pub fn reset(&self, protocol_version: i32) {
        if self.protocol_version != protocol_version {
            println!("Can't switch protocol version, when resetting the world :(");
        }
        self.modded_block_ids.clone().write().unwrap().clear();
        self.light_updates.clone().write().unwrap().clear();
        self.render_list.clone().write().unwrap().clear();
        self.block_entity_actions.clone().write().unwrap().clear();
        self.chunks.clone().write().unwrap().clear();
    }

    pub fn is_chunk_loaded(&self, x: i32, z: i32) -> bool {
        self.chunks.clone().read().unwrap().contains_key(&CPos(x, z))
    }

    pub fn set_block(&self, pos: Position, b: block::Block) {
        if self.set_block_raw(pos, b) {
            self.update_block(pos);
        }
    }

    fn set_block_raw(&self, pos: Position, b: block::Block) -> bool {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let chunks = self.chunks.clone();
        let mut chunks = chunks.write().unwrap();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        if chunk.set_block(pos.x & 0xF, pos.y, pos.z & 0xF, b) {
            if chunk.block_entities.contains_key(&pos) {
                self.block_entity_actions.clone().write().unwrap()
                    .push_back(BlockEntityAction::Remove(pos));
            }
            if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                self.block_entity_actions.clone().write().unwrap()
                    .push_back(BlockEntityAction::Create(pos));
            }
            true
        } else {
            false
        }
    }

    pub fn update_block(&self, pos: Position) {
        for yy in -1..2 {
            for zz in -1..2 {
                for xx in -1..2 {
                    let bp = pos + (xx, yy, zz);
                    let current = self.get_block(bp);
                    let new = current.update_state(self, bp);
                    if current != new {
                        self.set_block_raw(bp, new);
                    }
                    self.set_dirty(bp.x >> 4, bp.y >> 4, bp.z >> 4);
                    self.update_light(bp, LightType::Block);
                    self.update_light(bp, LightType::Sky);
                }
            }
        }
    }

    fn update_range(&self, x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) {
        for by in y1..y2 {
            for bz in z1..z2 {
                for bx in x1..x2 {
                    let bp = Position::new(bx, by, bz);
                    let current = self.get_block(bp);
                    let new = current.update_state(self, bp);
                    let sky_light = self.get_sky_light(bp);
                    let block_light = self.get_block_light(bp);
                    if current != new {
                        self.set_block_raw(bp, new);
                        // Restore old lighting
                        self.set_sky_light(bp, sky_light);
                        self.set_block_light(bp, block_light);
                    }
                }
            }
        }
    }

    pub fn get_block(&self, pos: Position) -> block::Block {
        match self.chunks.clone().read().unwrap().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_block(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => block::Missing {},
        }
    }

    fn set_block_light(&self, pos: Position, light: u8) {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let chunks = self.chunks.clone();
        let mut chunks = chunks.write().unwrap();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        chunk.set_block_light(pos.x & 0xF, pos.y, pos.z & 0xF, light);
    }

    pub fn get_block_light(&self, pos: Position) -> u8 {
        match self.chunks.clone().read().unwrap().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_block_light(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => 0,
        }
    }

    fn set_sky_light(&self, pos: Position, light: u8) {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let chunks = self.chunks.clone();
        let mut chunks = chunks.write().unwrap();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        chunk.set_sky_light(pos.x & 0xF, pos.y, pos.z & 0xF, light);
    }

    pub fn get_sky_light(&self, pos: Position) -> u8 {
        match self.chunks.clone().read().unwrap().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_sky_light(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => 15,
        }
    }

    fn update_light(&self, pos: Position, ty: LightType) {
        self.light_updates.clone().write().unwrap().push_back(LightUpdate { ty, pos });
    }

    pub fn add_block_entity_action(&self, action: BlockEntityAction) {
        self.block_entity_actions.clone().write().unwrap().push_back(action);
    }

    #[allow(clippy::verbose_bit_mask)] // "llvm generates better code" for updates_performed & 0xFFF "on x86"
    pub fn tick(&self, m: &mut ecs::Manager) {
        let sign_info: ecs::Key<block_entity::sign::SignInfo> = m.get_key();
        let block_entity_actions = self.block_entity_actions.clone();
        let mut block_entity_actions = block_entity_actions.write().unwrap();
        'start: while let Some(action) = block_entity_actions.pop_front() {
            match action {
                BlockEntityAction::Remove(pos) => {
                    if let Ok(mut chunk) = self.chunks.clone().try_write() {
                        if let Some(chunk) = chunk.get_mut(&CPos(pos.x >> 4, pos.z >> 4)) {
                            if let Some(entity) = chunk.block_entities.remove(&pos) {
                                m.remove_entity(entity);
                            }
                        }
                    } else {
                        block_entity_actions.push_front(action);
                        break 'start;
                    }
                }
                BlockEntityAction::Create(pos) => {
                    if let Ok(mut chunk) = self.chunks.clone().try_write() {
                        if let Some(chunk) = chunk.get_mut(&CPos(pos.x >> 4, pos.z >> 4)) {
                            // Remove existing entity
                            if let Some(entity) = chunk.block_entities.remove(&pos) {
                                m.remove_entity(entity);
                            }
                            let block = chunk.get_block(pos.x & 0xF, pos.y, pos.z & 0xF);
                            if let Some(entity_type) =
                            block_entity::BlockEntityType::get_block_entity(block)
                            {
                                let entity = entity_type.create_entity(m, pos);
                                chunk.block_entities.insert(pos, entity);
                            }
                        }
                    } else {
                        block_entity_actions.push_front(action);
                        break 'start;
                    }
                }
                BlockEntityAction::UpdateSignText(bx) => {
                    if let Ok(chunk) = self.chunks.clone().try_read() {
                        let (pos, line1, line2, line3, line4) = *bx;
                        if let Some(chunk) = chunk.get(&CPos(pos.x >> 4, pos.z >> 4)) {
                            if let Some(entity) = chunk.block_entities.get(&pos) {
                                if let Some(sign) = m.get_component_mut(*entity, sign_info) {
                                    sign.lines = [line1, line2, line3, line4];
                                    sign.dirty = true;
                                }
                            }
                        }
                    } else {
                        block_entity_actions.push_front(BlockEntityAction::UpdateSignText(bx));
                        break 'start;
                    }
                }
            }
        }
    }

    pub(crate) fn do_light_update(&self) {
        use std::cmp;
        let update = self.light_updates.clone();
        let mut _update = update.write().unwrap();
        let update = _update.pop_front();
        drop(_update);
        if let Some(update) = update {
            if update.pos.y < 0
                || update.pos.y > 255
                || !self.is_chunk_loaded(update.pos.x >> 4, update.pos.z >> 4)
            {
                return;
            }

            let block = self.get_block(update.pos).get_material();
            // Find the brightest source of light nearby
            let mut best = update.ty.get_light(self, update.pos);
            let old = best;
            for dir in Direction::all() {
                let light = update.ty.get_light(self, update.pos.shift(dir));
                if light > best {
                    best = light;
                }
            }
            best = best.saturating_sub(cmp::max(1, block.absorbed_light));
            // If the light from the block itself is brighter than the light passing through
            // it use that.
            if update.ty == LightType::Block && block.emitted_light != 0 {
                best = cmp::max(best, block.emitted_light);
            }
            // Sky light doesn't decrease when going down at full brightness
            if update.ty == LightType::Sky
                && block.absorbed_light == 0
                && update.ty.get_light(self, update.pos.shift(Direction::Up)) == 15
            {
                best = 15;
            }

            // Nothing to do, we are already at the right value
            if best == old {
                return;
            }
            // Use our new light value
            update.ty.set_light(self, update.pos, best);
            // Flag surrounding chunks as dirty
            for yy in -1..2 {
                for zz in -1..2 {
                    for xx in -1..2 {
                        let bp = update.pos + (xx, yy, zz);
                        self.set_dirty(bp.x >> 4, bp.y >> 4, bp.z >> 4);
                    }
                }
            }

            // Update surrounding blocks
            for dir in Direction::all() {
                self.update_light(update.pos.shift(dir), update.ty);
            }
        }
    }

    pub fn copy_cloud_heightmap(&self, data: &mut [u8]) -> bool {
        let mut dirty = false;
        for c in self.chunks.clone().write().unwrap().values_mut() {
            if c.heightmap_dirty {
                dirty = true;
                c.heightmap_dirty = false;
                for xx in 0..16 {
                    for zz in 0..16 {
                        data[(((c.position.0 << 4) as usize + xx) & 0x1FF)
                            + ((((c.position.1 << 4) as usize + zz) & 0x1FF) << 9)] =
                            c.heightmap[(zz << 4) | xx];
                    }
                }
            }
        }
        dirty
    }

    pub fn compute_render_list(&self, renderer: /*&mut */Arc<RwLock<render::Renderer>>) {
        let start_rec = Instant::now();
        // self.render_list.clone().write().unwrap().clear(); // TODO: Sync with the main thread somehow!
        // renderer.clone().read().unwrap()

        let mut valid_dirs = [false; 6];
        for dir in Direction::all() {
            let (ox, oy, oz) = dir.get_offset();
            let dir_vec = cgmath::Vector3::new(ox as f32, oy as f32, oz as f32);
            valid_dirs[dir.index()] = renderer.clone().read().unwrap().view_vector.dot(dir_vec) > -0.9;
        }

        let start = (
            ((renderer.clone().read().unwrap().camera.pos.x as i32) >> 4),
            ((renderer.clone().read().unwrap().camera.pos.y as i32) >> 4),
            ((renderer.clone().read().unwrap().camera.pos.z as i32) >> 4),
        );

        let mut render_queue = Arc::new(RwLock::new(Vec::new()));
        let mut process_queue = VecDeque::with_capacity(self.chunks.clone().read().unwrap().len() * 16);
        // println!("processqueue size {}", self.chunks.len() * 16);
        process_queue.push_front((Direction::Invalid, start));
        let diff = Instant::now().duration_since(start_rec);
        println!("Delay took {}", diff.as_millis());
        let frustum = renderer.clone().read().unwrap().frustum.clone();
        let frame_id = renderer.clone().read().unwrap().frame_id.clone();
        self.do_render_queue(Arc::new(RwLock::new(process_queue)),
                             frustum, frame_id, valid_dirs, render_queue.clone());
        let render_list_write = self.render_list.clone();
        let mut render_list_write = render_list_write.write().unwrap();
        render_list_write.clear();
        render_list_write.extend(render_queue.clone().read().unwrap().iter());

        // TODO: Improve the performance of the following by moving this to another thread!
        /*
        process_queue.par_iter().for_each(|(from, pos)| {
            let (exists, cull) = if let Some((sec, rendered_on)) =
            self.get_render_section_mut(pos.0, pos.1, pos.2)
            {
                if rendered_on == renderer.frame_id {
                    return;
                }
                if let Some(chunk) = self.chunks.clone().write().unwrap().get_mut(&CPos(pos.0, pos.2)) {
                    chunk.sections_rendered_on[pos.1 as usize] = renderer.frame_id;
                }

                let min = cgmath::Point3::new(
                    pos.0 as f32 * 16.0,
                    -pos.1 as f32 * 16.0,
                    pos.2 as f32 * 16.0,
                );
                let bounds =
                    collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
                if renderer.frustum.contains(&bounds) == collision::Relation::Out
                    && *from != Direction::Invalid
                {
                    return;
                }
                (
                    sec.is_some(),
                    sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v.clone().read().unwrap().cull_info),
                )
            } else {
                return;
            };

            if exists {
                self.render_list.clone().write().unwrap().push(*pos);
            }

            for dir in Direction::all() {
                let (ox, oy, oz) = dir.get_offset();
                let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
                if let Some((_, rendered_on)) = self.get_render_section_mut(opos.0, opos.1, opos.2)
                {
                    if rendered_on == renderer.frame_id {
                        continue;
                    }
                    if *from == Direction::Invalid
                        || (valid_dirs[dir.index()] && cull.is_visible(*from, dir))
                    {
                        process_queue.push_back((dir.opposite(), opos));
                    }
                }
            }
        });*/

        /*while let Some((from, pos)) = process_queue.pop_front() { // TODO: Use par iters
            let (exists, cull) = if let Some((sec, rendered_on)) =
                self.get_render_section_mut(pos.0, pos.1, pos.2)
            {
                if rendered_on == renderer.frame_id {
                    continue;
                }
                if let Some(chunk) = self.chunks.clone().write().unwrap().get_mut(&CPos(pos.0, pos.2)) {
                    chunk.sections_rendered_on[pos.1 as usize] = renderer.frame_id;
                }

                let min = cgmath::Point3::new(
                    pos.0 as f32 * 16.0,
                    -pos.1 as f32 * 16.0,
                    pos.2 as f32 * 16.0,
                );
                let bounds =
                    collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
                if renderer.frustum.contains(&bounds) == collision::Relation::Out
                    && from != Direction::Invalid
                {
                    continue;
                }
                (
                    sec.is_some(),
                    sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v.clone().read().unwrap().cull_info),
                )
            } else {
                continue;
            };

            if exists {
                self.render_list.clone().write().unwrap().push(pos);
            }

            for dir in Direction::all() {
                let (ox, oy, oz) = dir.get_offset();
                let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
                if let Some((_, rendered_on)) = self.get_render_section_mut(opos.0, opos.1, opos.2)
                {
                    if rendered_on == renderer.frame_id {
                        continue;
                    }
                    if from == Direction::Invalid
                        || (valid_dirs[dir.index()] && cull.is_visible(from, dir))
                    {
                        process_queue.push_back((dir.opposite(), opos));
                    }
                }
            }
        }*/
    }

    fn do_render_queue(&self, process_queue: Arc<RwLock<VecDeque<(Direction, (i32, i32, i32))>>>,
                       frustum: Frustum<f32>, frame_id: u32, valid_dirs: [bool; 6], render_queue: Arc<RwLock<Vec<(i32, i32, i32)>>>) {
        let out = Arc::new(RwLock::new(VecDeque::new())); // TODO: Add Arc!
        /*let tmp_renderer = renderer.clone();
        let tmp_renderer = tmp_renderer.read().unwrap();
        let frame_id = tmp_renderer.frame_id.clone();*/
        // let frame_id = renderer.clone().read().unwrap().frame_id.clone();
        // let frustum = renderer.clone().read().unwrap().frustum.clone().read().unwrap().as_ref().unwrap();
        let tmp_frustum = frustum.clone();
        // println!("rendering {} elems", process_queue.clone().read().unwrap().len());
        process_queue.clone().read().unwrap().iter().for_each(|(from, pos)| {
            let (exists, cull) = if let Some((sec, rendered_on)) =
            self.get_render_section_mut(pos.0, pos.1, pos.2)
            {
                if rendered_on == frame_id {
                    return;
                }
                if let Some(chunk) = self.chunks.clone().write().unwrap().get_mut(&CPos(pos.0, pos.2)) {
                    chunk.sections_rendered_on[pos.1 as usize] = frame_id;
                }

                let min = cgmath::Point3::new(
                    pos.0 as f32 * 16.0,
                    -pos.1 as f32 * 16.0,
                    pos.2 as f32 * 16.0,
                );
                let bounds =
                    collision::Aabb3::new(min, min + cgmath::Vector3::new(16.0, -16.0, 16.0));
                if tmp_frustum.contains(&bounds) == collision::Relation::Out
                    && *from != Direction::Invalid
                {
                    return;
                }
                (
                    sec.is_some(),
                    sec.map_or(chunk_builder::CullInfo::all_vis(), |v| v),
                )
            } else {
                return;
            };

            if exists {
                render_queue.clone().write().unwrap().push(*pos);
            }

            for dir in Direction::all() {
                let (ox, oy, oz) = dir.get_offset();
                let opos = (pos.0 + ox, pos.1 + oy, pos.2 + oz);
                if let Some((_, rendered_on)) = self.get_render_section_mut(opos.0, opos.1, opos.2)
                {
                    if rendered_on == frame_id {
                        continue;
                    }
                    if *from == Direction::Invalid
                        || (valid_dirs[dir.index()] && cull.is_visible(*from, dir))
                    {
                        out.clone().write().unwrap().push_back((dir.opposite(), opos));
                    }
                }
            }
        });
        if !out.clone().read().unwrap().is_empty() {
            self.do_render_queue(out.clone(), frustum.clone(), frame_id, valid_dirs, render_queue);
        } else {
            println!("finished!");
        }
    }

    pub fn get_render_list(&self) -> Vec<((i32, i32, i32), Arc<RwLock<render::ChunkBuffer>>)> {
        self.render_list.clone().read().unwrap()
            .iter()
            // .par_iter()
            .filter_map(|v| {
                let chunks = self.chunks.clone();
                let chunks = chunks.read().unwrap();
                let chunk = chunks.get(&CPos(v.0, v.2));
                if let Some(chunk) = chunk {
                    if let Some(sec) = chunk.sections[v.1 as usize].as_ref() {
                        let sec = sec.clone();
                        let sec = sec.read().unwrap();
                        return Some((*v, sec.render_buffer.clone()));
                    }
                }
                return None;
            })
            .collect()
    }
    /*
    thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', src/world/mod.rs:414:62
stack backtrace:
   0: rust_begin_unwind
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/std/src/panicking.rs:493:5
   1: core::panicking::panic_fmt
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/panicking.rs:92:14
   2: core::panicking::panic
             at /rustc/53cb7b09b00cbea8754ffb78e7e3cb521cb8af4b/library/core/src/panicking.rs:50:5
   3: core::option::Option<T>::unwrap
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:386:21
   4: leafish::world::World::get_render_list::{{closure}}
             at /home/threadexception/IdeaProjects/Leafish/src/world/mod.rs:414:29
   5: core::iter::adapters::map::map_fold::{{closure}}
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/map.rs:82:28
   6: core::iter::traits::iterator::Iterator::fold
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:2146:21
   7: <core::iter::adapters::map::Map<I,F> as core::iter::traits::iterator::Iterator>::fold
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/adapters/map.rs:122:9
   8: core::iter::traits::iterator::Iterator::for_each
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:776:9
   9: <alloc::vec::Vec<T,A> as alloc::vec::spec_extend::SpecExtend<T,I>>::spec_extend
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/spec_extend.rs:40:17
  10: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/spec_from_iter_nested.rs:56:9
  11: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter::SpecFromIter<T,I>>::from_iter
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/spec_from_iter.rs:36:9
  12: <alloc::vec::Vec<T> as core::iter::traits::collect::FromIterator<T>>::from_iter
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec/mod.rs:2404:9
  13: core::iter::traits::iterator::Iterator::collect
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/traits/iterator.rs:1788:9
  14: leafish::world::World::get_render_list
             at /home/threadexception/IdeaProjects/Leafish/src/world/mod.rs:411:9
  15: leafish::chunk_builder::ChunkBuilder::tick
             at /home/threadexception/IdeaProjects/Leafish/src/chunk_builder.rs:97:30
  16: leafish::tick_all
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:507:5
  17: leafish::main::{{closure}}
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:423:9
  18: winit::platform_impl::platform::sticky_exit_callback
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/mod.rs:746:5
  19: winit::platform_impl::platform::wayland::event_loop::EventLoop<T>::run_return
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/wayland/event_loop/mod.rs:354:13
  20: winit::platform_impl::platform::wayland::event_loop::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/wayland/event_loop/mod.rs:191:9
  21: winit::platform_impl::platform::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/mod.rs:662:56
  22: winit::event_loop::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/event_loop.rs:154:9
  23: leafish::main
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:403:5
  24: core::ops::function::FnOnce::call_once
             at /home/threadexception/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ops/function.rs:227:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

Process finished with exit code 101
     */
    /*
    rendering 179 elems
thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', src/world/mod.rs:590:57
stack backtrace:
   0: rust_begin_unwind
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/std/src/panicking.rs:515:5
   1: core::panicking::panic_fmt
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/panicking.rs:92:14
   2: core::panicking::panic
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/panicking.rs:50:5
   3: core::option::Option<T>::unwrap
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/option.rs:388:21
   4: leafish::world::World::get_render_list::{{closure}}
             at /home/threadexception/IdeaProjects/Leafish/src/world/mod.rs:590:29
   5: core::iter::adapters::map::map_fold::{{closure}}
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/iter/adapters/map.rs:82:28
   6: core::iter::traits::iterator::Iterator::fold
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/iter/traits/iterator.rs:2112:21
   7: <core::iter::adapters::map::Map<I,F> as core::iter::traits::iterator::Iterator>::fold
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/iter/adapters/map.rs:122:9
   8: core::iter::traits::iterator::Iterator::for_each
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/iter/traits/iterator.rs:736:9
   9: <alloc::vec::Vec<T,A> as alloc::vec::spec_extend::SpecExtend<T,I>>::spec_extend
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/alloc/src/vec/spec_extend.rs:40:17
  10: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/alloc/src/vec/spec_from_iter_nested.rs:56:9
  11: <alloc::vec::Vec<T> as alloc::vec::spec_from_iter::SpecFromIter<T,I>>::from_iter
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/alloc/src/vec/spec_from_iter.rs:33:9
  12: <alloc::vec::Vec<T> as core::iter::traits::collect::FromIterator<T>>::from_iter
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/alloc/src/vec/mod.rs:2449:9
  13: core::iter::traits::iterator::Iterator::collect
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/iter/traits/iterator.rs:1748:9
  14: leafish::world::World::get_render_list
             at /home/threadexception/IdeaProjects/Leafish/src/world/mod.rs:584:9
  15: leafish::chunk_builder::ChunkBuilder::tick
             at /home/threadexception/IdeaProjects/Leafish/src/chunk_builder.rs:96:30
  16: leafish::tick_all
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:526:9
  17: leafish::main::{{closure}}
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:437:9
  18: winit::platform_impl::platform::sticky_exit_callback
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/mod.rs:746:5
  19: winit::platform_impl::platform::wayland::event_loop::EventLoop<T>::run_return
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/wayland/event_loop/mod.rs:354:13
  20: winit::platform_impl::platform::wayland::event_loop::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/wayland/event_loop/mod.rs:191:9
  21: winit::platform_impl::platform::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/platform_impl/linux/mod.rs:662:56
  22: winit::event_loop::EventLoop<T>::run
             at /home/threadexception/.cargo/registry/src/github.com-1ecc6299db9ec823/winit-0.25.0/src/event_loop.rs:154:9
  23: leafish::main
             at /home/threadexception/IdeaProjects/Leafish/src/main.rs:416:5
  24: core::ops::function::FnOnce::call_once
             at /rustc/a178d0322ce20e33eac124758e837cbd80a6f633/library/core/src/ops/function.rs:227:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
do next!
rendering 198 elems

Process finished with exit code 101
     */

    pub fn get_section_mut(&self, x: i32, y: i32, z: i32) -> Option<Arc<RwLock<Section>>> {
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(x, z)) {
            if let Some(sec) = chunk.sections[y as usize].as_ref() {
                return Some(sec.clone());
            }
        }
        None
    }

    // TODO: Improve the perf of this method as it is the MAIN bottleneck slowing down the program!
    fn get_render_section_mut(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<(Option<CullInfo>, u32)> {
        if !(0..=15).contains(&y) {
            return None;
        }
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(x, z)) {
            let rendered = &chunk.sections_rendered_on[y as usize];
            if let Some(sec) = chunk.sections[y as usize].as_ref() {
                return Some((Some(sec.clone().read().unwrap().cull_info), rendered.clone()));
            }
            return Some((None, rendered.clone()));
        }
        None
    }

    pub fn get_dirty_chunk_sections(&self) -> Vec<(i32, i32, i32)> {
        let mut out = vec![];
        for chunk in self.chunks.clone().read().unwrap().values() {
            for sec in &chunk.sections {
                if let Some(sec) = sec.as_ref() {
                    if !sec.clone().read().unwrap().building && sec.clone().read().unwrap().dirty {
                        out.push((chunk.position.0, sec.clone().read().unwrap().y as i32, chunk.position.1));
                    }
                }
            }
        }
        out
    }

    fn set_dirty(&self, x: i32, y: i32, z: i32) {
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(x, z)) {
            if let Some(sec) = chunk.sections.get(y as usize).and_then(|v| v.as_ref()) {
                sec.clone().write().unwrap().dirty = true;
            }
        }
    }

    pub fn is_section_dirty(&self, pos: (i32, i32, i32)) -> bool {
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(pos.0, pos.2)) {
            if let Some(sec) = chunk.sections[pos.1 as usize].as_ref() {
                return sec.clone().read().unwrap().dirty && !sec.clone().read().unwrap().building;
            }
        }
        false
    }

    pub fn set_building_flag(&self, pos: (i32, i32, i32)) {
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(pos.0, pos.2)) {
            if let Some(sec) = chunk.sections[pos.1 as usize].as_ref() {
                sec.clone().write().unwrap().building = true;
                sec.clone().write().unwrap().dirty = false;
            }
        }
    }

    pub fn reset_building_flag(&self, pos: (i32, i32, i32)) {
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&CPos(pos.0, pos.2)) {
            if let Some(section) = chunk.sections[pos.1 as usize].as_ref() {
                section.clone().write().unwrap().building = false;
            }
        }
    }

    pub fn flag_dirty_all(&self) {
        for chunk in self.chunks.clone().read().unwrap().values() {
            for sec in &chunk.sections {
                if let Some(sec) = sec.as_ref() {
                    sec.clone().write().unwrap().dirty = true;
                }
            }
        }
    }

    pub fn capture_snapshot(&self, x: i32, y: i32, z: i32,) -> Option<SectionSnapshot> { // TODO: Improve performance!
        let cx = x >> 4;
        let cy = y >> 4;
        let cz = z >> 4;
        let chunks = self.chunks.clone();
        let chunks = chunks.read().unwrap();
        let chunk = match chunks.get(&CPos(cx, cz)) {
            Some(val) => val,
            None => {
                return None;
            },
        };
        let sec = &chunk.sections[cy as usize];
        if sec.is_none() {
            return None;
        }
        return Some(sec.as_ref().unwrap().clone().read().unwrap().capture_snapshot(chunk.biomes.clone()));
    }

    pub fn unload_chunk(&self, x: i32, z: i32, m: &mut ecs::Manager) {
        if let Some(chunk) = self.chunks.clone().write().unwrap().remove(&CPos(x, z)) {
            for entity in chunk.block_entities.values() {
                m.remove_entity(*entity);
            }
        }
    }

    pub fn load_chunk(&self,
                      x: i32,
                      z: i32,
                      new: bool,
                      skylight: bool,
                      read_biomes: bool,
                      mask: u16,
                      mask_add: u16,
                      data: &mut Cursor<Vec<u8>>,
                      version: u8) -> Result<(), protocol::Error> {
        let additional_light_data = self.lighting_cache.clone().write().unwrap().remove(&CPos(x, z));
        let has_add_light = additional_light_data.is_some();
        let mut additional_light_data = additional_light_data.unwrap();
        let cpos = CPos(x, z);
        {
            if new {
                // TODO: Improve lighting with something similar to bixilon's light accessor!
                self.chunks.clone().write().unwrap().insert(cpos, Chunk::new(cpos));
            } else if !self.chunks.clone().read().unwrap().contains_key(&cpos) {
                return Ok(());
            }
            let chunks = self.chunks.clone();
            let mut chunks = chunks.write().unwrap();
            let chunk = chunks.get_mut(&cpos).unwrap();

            // Block type array - whole byte per block  // 17
            let mut block_types: [[u8; 4096]; 16] = [[0u8; 4096]; 16]; // 17
            for i in 0..16 {
                if chunk.sections[i].is_none() {
                    let mut fill_sky = chunk.sections.iter().skip(i).all(|v| v.is_none());
                    fill_sky &= (mask & !((1 << i) | ((1 << i) - 1))) == 0;
                    if !fill_sky || mask & (1 << i) != 0 {
                        chunk.sections[i] = Some(Arc::new(RwLock::new(Section::new(i as u8, fill_sky))));
                    }
                }
                if mask & (1 << i) == 0 {
                    continue;
                }
                let section = chunk.sections[i as usize].as_ref().unwrap();

                if version == 17 {
                    data.read_exact(&mut block_types[i])?;
                } else if version == 18 {
                    self.prep_section_18(chunk, section.clone(), data, i);
                } else if version == 19 {
                    self.prep_section_19(chunk, section.clone(), data, i);
                }
                section.clone().write().unwrap().dirty = true;
            }
            if version == 17 {
                self.finish_17(chunk, mask, mask_add, skylight, data, block_types);
            } else if version != 19 {
                self.read_light(chunk, mask, skylight, data);
            } else if has_add_light {
                self.load_light(chunk, additional_light_data.block_light_mask, true, additional_light_data.sky_light_mask, &mut additional_light_data.arrays);
            }

            if new && read_biomes { // read biomes is always true (as param) except for load_chunk_19
                data.read_exact(&mut chunk.biomes)?;
            }

            chunk.calculate_heightmap();
        }

        self.dirty_chunks_by_bitmask(x, z, mask);
        Ok(())
    }

    fn prep_section_19(&self, chunk: &Chunk, section: Arc<RwLock<Section>>, data: &mut Cursor<Vec<u8>>, section_id: usize) {
        use crate::protocol::{LenPrefixed, Serializable, VarInt};
        if self.protocol_version >= 451 {
            let _block_count = data.read_u16::<byteorder::LittleEndian>().unwrap();
            // TODO: use block_count
        }

        let mut bit_size = data.read_u8().unwrap();
        let mut mappings: HashMap<usize, block::Block, BuildHasherDefault<FNVHash>> =
            HashMap::with_hasher(BuildHasherDefault::default());
        if bit_size == 0 {
            bit_size = 13;
        } else {
            let count = VarInt::read_from(data).unwrap().0;
            for i in 0..count {
                let id = VarInt::read_from(data).unwrap().0;
                let bl = self
                    .id_map
                    .by_vanilla_id(id as usize, self.modded_block_ids.clone());
                mappings.insert(i as usize, bl);
            }
        }

        let bits = LenPrefixed::<VarInt, u64>::read_from(data).unwrap().data;
        let padded = self.protocol_version >= 736;
        let m = bit::Map::from_raw(bits, bit_size as usize, padded);

        for bi in 0..4096 {
            let id = m.get(bi);
            section.clone().write().unwrap().blocks.set(
                bi,
                mappings
                    .get(&id)
                    .cloned()
                    // TODO: fix or_fun_call, but do not re-borrow self
                    .unwrap_or(self.id_map.by_vanilla_id(id, self.modded_block_ids.clone())),
            );
            // Spawn block entities
            let b = section.clone().read().unwrap().blocks.get(bi);
            if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                let pos = Position::new(
                    (bi & 0xF) as i32,
                    (bi >> 8) as i32,
                    ((bi >> 4) & 0xF) as i32,
                ) + (
                    chunk.position.0 << 4,
                    (section_id << 4) as i32,
                    chunk.position.1 << 4,
                );
                if chunk.block_entities.contains_key(&pos) {
                    self.block_entity_actions.clone().write().unwrap()
                        .push_back(BlockEntityAction::Remove(pos))
                }
                self.block_entity_actions.clone().write().unwrap()
                    .push_back(BlockEntityAction::Create(pos))
            }
        }
        if self.protocol_version >= 451 {
            // Skylight in update skylight packet for 1.14+
        } else {
            data.read_exact(&mut section.clone().write().unwrap().block_light.data).unwrap();
            data.read_exact(&mut section.clone().write().unwrap().sky_light.data).unwrap();
        }
    }

    fn prep_section_18(&self, chunk: &Chunk, section: Arc<RwLock<Section>>, data: &mut Cursor<Vec<u8>>, section_id: usize) {
        for bi in 0..4096 {
            let id = data.read_u16::<byteorder::LittleEndian>().unwrap();
            section.clone().write().unwrap().blocks.set(
                bi,
                self.id_map
                    .by_vanilla_id(id as usize, self.modded_block_ids.clone()),
            );

            // Spawn block entities
            let b = section.clone().write().unwrap().blocks.get(bi);
            if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                let pos = Position::new(
                    (bi & 0xF) as i32,
                    (bi >> 8) as i32,
                    ((bi >> 4) & 0xF) as i32,
                ) + (
                    chunk.position.0 << 4,
                    (section_id << 4) as i32,
                    chunk.position.1 << 4,
                );
                if chunk.block_entities.contains_key(&pos) {
                    self.block_entity_actions.clone().write().unwrap()
                        .push_back(BlockEntityAction::Remove(pos))
                }
                self.block_entity_actions.clone().write().unwrap()
                    .push_back(BlockEntityAction::Create(pos))
            }
        }
    }

    fn read_light(&self, chunk: &Chunk, mask: u16, skylight: bool, data: &mut Cursor<Vec<u8>>) {
        // Block light array - half byte per block
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let section = chunk.sections[i as usize].as_ref().unwrap();

            data.read_exact(&mut section.clone().write().unwrap().block_light.data).unwrap();
        }

        // Sky light array - half byte per block - only if 'skylight' is true
        if skylight {
            for i in 0..16 {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let section = chunk.sections[i as usize].as_ref().unwrap();

                data.read_exact(&mut section.clone().write().unwrap().sky_light.data).unwrap();
            }
        }
    }

    fn finish_17(&self, chunk: &Chunk, mask: u16, mask_add: u16, skylight: bool, data: &mut Cursor<Vec<u8>>, block_types: [[u8; 4096]; 16]) {
        // Block metadata array - half byte per block
        let mut block_meta: [nibble::Array; 16] = [
            // TODO: cleanup this initialization
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
        ];

        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }

            data.read_exact(&mut block_meta[i].data).unwrap();
        }

        self.read_light(chunk, mask, skylight, data);

        // Add array - half byte per block - uses secondary bitmask
        let mut block_add: [nibble::Array; 16] = [
            // TODO: cleanup this initialization
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
            nibble::Array::new(16 * 16 * 16),
        ];

        for i in 0..16 {
            if mask_add & (1 << i) == 0 {
                continue;
            }
            data.read_exact(&mut block_add[i].data).unwrap();
        }

        // Now that we have the block types, metadata, and add, combine to initialize the blocks
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }

            let section = chunk.sections[i as usize].as_ref().unwrap();

            for bi in 0..4096 {
                let id = ((block_add[i].get(bi) as u16) << 12)
                    | ((block_types[i][bi] as u16) << 4)
                    | (block_meta[i].get(bi) as u16);
                section.clone().write().unwrap().blocks.set(
                    bi,
                    self.id_map
                        .by_vanilla_id(id as usize, self.modded_block_ids.clone()),
                );

                // Spawn block entities
                let b = section.clone().read().unwrap().blocks.get(bi);
                if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                    let pos = Position::new(
                        (bi & 0xF) as i32,
                        (bi >> 8) as i32,
                        ((bi >> 4) & 0xF) as i32,
                    ) + (
                        chunk.position.0 << 4,
                        (i << 4) as i32,
                        chunk.position.1 << 4,
                    );
                    if chunk.block_entities.contains_key(&pos) {
                        self.block_entity_actions.clone().write().unwrap()
                            .push_back(BlockEntityAction::Remove(pos))
                    }
                    self.block_entity_actions.clone().write().unwrap()
                        .push_back(BlockEntityAction::Create(pos))
                }
            }
        }
    }

    /*
    pub fn load_chunks(&self,
                       skylight: bool,
                       chunk_column_count: u16, // 17
                       data_length: i32, // 17
                       new: bool, // 18, 19
                       read_biomes: bool, // 19
                       chunk_metas: &[crate::protocol::packet::ChunkMeta], // 18
                       mask: u16, // 19
                       data: Vec<u8>) -> Result<(), protocol::Error> { // Vec<u8> | &[u8]

    }*/

    pub fn load_chunks18(
        &self,
        new: bool,
        skylight: bool,
        chunk_metas: &[crate::protocol::packet::ChunkMeta],
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        let mut data = std::io::Cursor::new(data);

        for chunk_meta in chunk_metas {
            let x = chunk_meta.x;
            let z = chunk_meta.z;
            let mask = chunk_meta.bitmask;

            self.load_chunk18(x, z, new, skylight, mask, &mut data)?;
        }
        Ok(())
    }

    fn dirty_chunks_by_bitmask(&self, x: i32, z: i32, mask: u16) {
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }
            for pos in [
                (-1, 0, 0),
                (1, 0, 0),
                (0, -1, 0),
                (0, 1, 0),
                (0, 0, -1),
                (0, 0, 1),
            ]
            .iter()
            {
                self.flag_section_dirty(x + pos.0, i as i32 + pos.1, z + pos.2);
            }
            self.update_range(
                (x << 4) - 1,
                (i << 4) - 1,
                (z << 4) - 1,
                (x << 4) + 17,
                (i << 4) + 17,
                (z << 4) + 17,
            );
        }
    }

    pub fn load_chunk18(
        &self,
        x: i32,
        z: i32,
        new: bool,
        _skylight: bool, // unused!
        mask: u16,
        data: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk(x, z, new, true, new, mask, 0, data, 18)
    }

    pub fn load_chunks17(
        &self,
        chunk_column_count: u16,
        data_length: i32,
        skylight: bool,
        data: &[u8],
    ) -> Result<(), protocol::Error> {
        let compressed_chunk_data = &data[0..data_length as usize];
        let metadata = &data[data_length as usize..];

        let mut zlib = ZlibDecoder::new(std::io::Cursor::new(compressed_chunk_data.to_vec()));
        let mut chunk_data = Vec::new();
        zlib.read_to_end(&mut chunk_data)?;

        let mut chunk_data = std::io::Cursor::new(chunk_data);

        // Chunk metadata
        let mut metadata = std::io::Cursor::new(metadata);
        for _i in 0..chunk_column_count {
            let x = metadata.read_i32::<byteorder::BigEndian>()?;
            let z = metadata.read_i32::<byteorder::BigEndian>()?;
            let mask = metadata.read_u16::<byteorder::BigEndian>()?;
            let mask_add = metadata.read_u16::<byteorder::BigEndian>()?;

            let new = true;

            self.load_uncompressed_chunk17(x, z, new, skylight, mask, mask_add, &mut chunk_data)?;
        }

        Ok(())
    }

    pub fn load_chunk17(
        &self,
        x: i32,
        z: i32,
        new: bool,
        mask: u16,
        mask_add: u16,
        compressed_data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        let mut zlib = ZlibDecoder::new(std::io::Cursor::new(compressed_data.to_vec()));
        let mut data = Vec::new();
        zlib.read_to_end(&mut data)?;

        let skylight = true;
        self.load_uncompressed_chunk17(
            x,
            z,
            new,
            skylight,
            mask,
            mask_add,
            &mut std::io::Cursor::new(data),
        )
    }

    #[allow(clippy::needless_range_loop)]
    fn load_uncompressed_chunk17(
        &self,
        x: i32,
        z: i32,
        new: bool,
        skylight: bool,
        mask: u16,
        mask_add: u16,
        data: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk(x, z, new, skylight, new, mask, mask_add, data, 17)
    }

    pub fn load_light_with_loc(&self, x: i32, z: i32, block_light_mask: i32, sky_light: bool, sky_light_mask: i32, data: &mut Cursor<Vec<u8>>) {
        // println!("x {} z {}", x, z);
        // TODO: Insert chunks with light data only or cache them until the real data arrives!
        /*let cpos = CPos(x, z);
        let chunks = self.chunks.clone();
        let mut chunks = chunks.write().unwrap();
        let chunk = chunks.get_mut(&cpos).unwrap(); // TODO: Fix this panic!
        self.load_light(chunk, block_light_mask, sky_light, sky_light_mask, data);*/
    }

    fn load_light(&self, chunk: &mut Chunk, block_light_mask: i32, sky_light: bool, sky_light_mask: i32, data: &mut Cursor<Vec<u8>>) {
        for i in 0..16 {
            if block_light_mask & (1 << i) == 0 {
                continue;
            }
            if chunk.sections[i as usize].as_ref().is_none() {
                chunk.sections[i as usize].replace(Arc::new(RwLock::new(Section::new(i, false))));
            }
            let mut section = chunk.sections[i as usize].as_mut().unwrap();

            data.read_exact(&mut section.clone().write().unwrap().block_light.data).unwrap();
        }

        if sky_light {
            for i in 0..16 {
                if sky_light_mask & (1 << i) == 0 {
                    continue;
                }
                if chunk.sections[i as usize].as_ref().is_none() {
                    chunk.sections[i as usize].replace(Arc::new(RwLock::new(Section::new(i, false))));
                }
                let mut section = chunk.sections[i as usize].as_mut().unwrap();

                data.read_exact(&mut section.clone().write().unwrap().sky_light.data).unwrap();
            }
        }
    }

    pub fn load_chunk19(
        &self,
        x: i32,
        z: i32,
        new: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk19_or_115(true, x, z, new, mask, data)
    }

    pub fn load_chunk115(
        &self,
        x: i32,
        z: i32,
        new: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk19_or_115(false, x, z, new, mask, data)
    }

    #[allow(clippy::or_fun_call)]
    fn load_chunk19_or_115(
        &self,
        read_biomes: bool,
        x: i32,
        z: i32,
        new: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk(x, z, new, true, read_biomes, mask, 0, &mut Cursor::new(data), 19)
    }

    fn flag_section_dirty(&self, x: i32, y: i32, z: i32) {
        if !(0..=15).contains(&y) {
            return;
        }
        let cpos = CPos(x, z);
        if let Some(chunk) = self.chunks.clone().read().unwrap().get(&cpos) {
            if let Some(sec) = chunk.sections[y as usize].as_ref() {
                sec.clone().write().unwrap().dirty = true;
            }
        }
    }
}

impl block::WorldAccess for World {
    fn get_block(&self, pos: Position) -> block::Block {
        World::get_block(self, pos)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct CPos(pub i32, pub i32);

pub struct Chunk {
    position: CPos,

    pub(crate) sections: [Option<Arc<RwLock<Section>>>; 16],
    sections_rendered_on: [u32; 16],
    biomes: [u8; 16 * 16],

    heightmap: [u8; 16 * 16],
    heightmap_dirty: bool,

    block_entities: HashMap<Position, ecs::Entity, BuildHasherDefault<FNVHash>>,
}

impl Chunk {
    fn new(pos: CPos) -> Chunk {
        Chunk {
            position: pos,
            sections: [
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None
            ],
            sections_rendered_on: [0; 16],
            biomes: [0; 16 * 16],
            heightmap: [0; 16 * 16],
            heightmap_dirty: true,
            block_entities: HashMap::with_hasher(BuildHasherDefault::default()),
        }
    }

    fn calculate_heightmap(&mut self) {
        for x in 0..16 {
            for z in 0..16 {
                let idx = ((z << 4) | x) as usize;
                for yy in 0..256 {
                    let sy = 255 - yy;
                    if let block::Air { .. } = self.get_block(x, sy, z) {
                        continue;
                    }
                    self.heightmap[idx] = sy as u8;
                    break;
                }
            }
        }
        self.heightmap_dirty = true;
    }

    fn set_block(&mut self, x: i32, y: i32, z: i32, b: block::Block) -> bool {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return false;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if let block::Air {} = b {
                return false;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(Arc::new(RwLock::new(Section::new(s_idx as u8, fill_sky))));
        }
        {
            let section = self.sections[s_idx as usize].as_mut().unwrap();
            if !section.clone().write().unwrap().set_block(x, y & 0xF, z, b) {
                return false;
            }
        }
        let idx = ((z << 4) | x) as usize;
        match self.heightmap[idx].cmp(&(y as u8)) {
            Ordering::Less => {
                self.heightmap[idx] = y as u8;
                self.heightmap_dirty = true;
            }
            Ordering::Equal => {
                // Find a new lowest
                for yy in 0..y {
                    let sy = y - yy - 1;
                    if let block::Air { .. } = self.get_block(x, sy, z) {
                        continue;
                    }
                    self.heightmap[idx] = sy as u8;
                    break;
                }
                self.heightmap_dirty = true;
            }
            Ordering::Greater => (),
        }
        true
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return block::Missing {};
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.clone().read().unwrap().get_block(x, y & 0xF, z),
            None => block::Air {},
        }
    }

    fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return 0;
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.clone().read().unwrap().get_block_light(x, y & 0xF, z),
            None => 0,
        }
    }

    fn set_block_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if light == 0 {
                return;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(Arc::new(RwLock::new(Section::new(s_idx as u8, fill_sky))));
        }
        if let Some(sec) = self.sections[s_idx].as_mut() {
            sec.clone().write().unwrap().set_block_light(x, y & 0xF, z, light)
        }
    }

    fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return 15;
        }
        match self.sections[s_idx as usize].as_ref() {
            Some(sec) => sec.clone().read().unwrap().get_sky_light(x, y & 0xF, z),
            None => 15,
        }
    }

    fn set_sky_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        let s_idx = y >> 4;
        if !(0..=15).contains(&s_idx) {
            return;
        }
        let s_idx = s_idx as usize;
        if self.sections[s_idx].is_none() {
            if light == 15 {
                return;
            }
            let fill_sky = self.sections.iter().skip(s_idx).all(|v| v.is_none());
            self.sections[s_idx] = Some(Arc::new(RwLock::new(Section::new(s_idx as u8, fill_sky))));
        }
        if let Some(sec) = self.sections[s_idx as usize].as_mut() {
            sec.clone().write().unwrap().set_sky_light(x, y & 0xF, z, light)
        }
    }

    fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        biome::Biome::by_id(self.biomes[((z << 4) | x) as usize] as usize)
    }

    pub fn capture_snapshot(&self) -> ChunkSnapshot {
        let mut snapshot_sections = [None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,];
        for section in self.sections.iter().enumerate() {
            if section.1.is_some() {
                snapshot_sections[section.0] = Some(section.1.as_ref().unwrap().clone().read().unwrap().capture_snapshot(self.biomes.clone()));
            }
        }
        ChunkSnapshot {
            position: self.position.clone(),
            sections: snapshot_sections,
            biomes: self.biomes.clone(),
            heightmap: self.heightmap.clone(),
        }
    }
}

pub struct ChunkSnapshot {

    pub position: CPos,
    pub sections: [Option<SectionSnapshot>; 16],
    pub biomes: [u8; 16 * 16],
    pub heightmap: [u8; 16 * 16],

}

pub struct Section {
    pub cull_info: chunk_builder::CullInfo,
    pub render_buffer: Arc<RwLock<render::ChunkBuffer>>,

    y: u8,

    blocks: storage::BlockStorage,

    block_light: nibble::Array,
    sky_light: nibble::Array,

    dirty: bool,
    building: bool,
}

impl Section {
    fn new(y: u8, fill_sky: bool) -> Self {
        let mut section = Section {
            cull_info: chunk_builder::CullInfo::all_vis(),
            render_buffer: Arc::new(RwLock::new(render::ChunkBuffer::new())),
            y,

            blocks: storage::BlockStorage::new(4096),

            block_light: nibble::Array::new(16 * 16 * 16),
            sky_light: nibble::Array::new(16 * 16 * 16),

            dirty: false,
            building: false,
        };
        if fill_sky {
            for i in 0..16 * 16 * 16 {
                section.sky_light.set(i, 0xF);
            }
        }
        section
    }

    pub fn capture_snapshot(&self, biomes: [u8; 16 * 16]) -> SectionSnapshot {
        SectionSnapshot {
            y: self.y,
            blocks: self.blocks.clone(),
            block_light: self.block_light.clone(),
            sky_light: self.sky_light.clone(),
            biomes,
        }
    }

    fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        self.blocks.get(((y << 8) | (z << 4) | x) as usize)
    }

    fn set_block(&mut self, x: i32, y: i32, z: i32, b: block::Block) -> bool {
        if self.blocks.set(((y << 8) | (z << 4) | x) as usize, b) {
            self.dirty = true;
            self.set_sky_light(x, y, z, 0); // TODO: Do we have to set this every time?
            self.set_block_light(x, y, z, 0);
            true
        } else {
            false
        }
    }

    fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.block_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    fn set_block_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.block_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }

    fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        self.sky_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    fn set_sky_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.sky_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }
}

pub struct SectionSnapshot {
    pub y: u8,
    pub blocks: storage::BlockStorage,
    pub block_light: nibble::Array,
    pub sky_light: nibble::Array,
    pub biomes: [u8; 16 * 16], // TODO: Remove this by using the chunk's biome!
}

impl SectionSnapshot {

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> block::Block {
        /*if x < 0 || y < 0 || z < 0 || x > 15 || y > 15 || z > 15 {
            println!("getting block at {}, {}, {}", x, y, z);
        }*/
        let x = x & 15;// TODO: Remove this as soon as OOB calls are fixed!
        let y = y & 15;
        let z = z & 15;
        self.blocks.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, b: block::Block) -> bool {
        // println!("get block at {}, {}, {}", x, y, z);
        if self.blocks.set(((y << 8) | (z << 4) | x) as usize, b) {
            self.set_sky_light(x, y, z, 0); // TODO: Do we have to set this every time?
            self.set_block_light(x, y, z, 0);
            true
        } else {
            false
        }
    }

    pub fn get_block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let x = x & 15;
        let y = y & 15;
        let z = z & 15;
        self.block_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn set_block_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.block_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }

    pub fn get_sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        let x = x & 15;
        let y = y & 15;
        let z = z & 15;
        self.sky_light.get(((y << 8) | (z << 4) | x) as usize)
    }

    pub fn set_sky_light(&mut self, x: i32, y: i32, z: i32, l: u8) {
        self.sky_light.set(((y << 8) | (z << 4) | x) as usize, l);
    }

    pub fn get_biome(&self, x: i32, z: i32) -> biome::Biome {
        let x = x & 15;
        let z = z & 15;
        biome::Biome::by_id(self.biomes[((z << 4) | x) as usize] as usize)
    }

}
