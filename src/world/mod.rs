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

use arc_swap::ArcSwap;
pub use leafish_blocks as block;
use leafish_protocol::nbt::NamedTag;

use crate::shared::Position;
use crate::{ecs, format};

use byteorder::ReadBytesExt;
use cgmath::InnerSpace;
use collision::Frustum;
use crossbeam_channel::{unbounded, Receiver, Sender};
use flate2::read::ZlibDecoder;
use instant::Instant;
use leafish_protocol::protocol;
use leafish_protocol::types::nibble;
use leafish_shared::direction::Direction;
use log::warn;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{BufRead, Cursor, Read};
use std::sync::Arc;

pub use self::{chunk::*, lighting::*};
use leafish_protocol::protocol::{Serializable, VarInt};
use std::sync::atomic::Ordering;

pub mod biome;
mod chunk;
mod lighting;
mod storage;

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

pub struct World {
    pub chunks: Arc<RwLock<BTreeMap<CPos, Chunk>>>,
    pub lighting_cache: Arc<RwLock<BTreeMap<CPos, LightData>>>,

    pub(crate) light_updates: Sender<LightUpdate>,

    block_entity_actions: (Sender<BlockEntityAction>, Receiver<BlockEntityAction>),

    protocol_version: i32,
    pub modded_block_ids: ArcSwap<HashMap<usize, String>>,
    pub id_map: Arc<block::VanillaIDMap>,

    pub dimension: ArcSwap<Dimension>,
}

impl World {
    pub fn new(protocol_version: i32, sender: Sender<LightUpdate>) -> Self {
        let id_map = Arc::new(block::VanillaIDMap::new(protocol_version));
        Self {
            chunks: Arc::new(Default::default()),
            lighting_cache: Arc::new(Default::default()),
            protocol_version,
            modded_block_ids: ArcSwap::new(Arc::new(Default::default())),
            id_map,
            light_updates: sender,
            block_entity_actions: unbounded(),
            dimension: ArcSwap::new(Arc::new(Default::default())),
        }
    }

    pub fn reset(&self, protocol_version: i32) {
        if self.protocol_version != protocol_version {
            warn!("Can't switch protocol version, when resetting the world :(");
        }
        // TODO: Check if we actually have to do anything here.
    }

    pub fn is_chunk_loaded(&self, x: i32, z: i32) -> bool {
        self.chunks.read().contains_key(&CPos(x, z))
    }

    pub fn set_block(&self, pos: Position, b: block::Block) {
        if self.set_block_raw(pos, b) {
            self.update_block(pos);
        }
    }

    fn set_block_raw(&self, pos: Position, b: block::Block) -> bool {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let mut chunks = self.chunks.write();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        if chunk.set_block(pos.x & 0xF, pos.y, pos.z & 0xF, b) {
            if chunk.block_entities.contains_key(&pos) {
                self.block_entity_actions
                    .0
                    .send(BlockEntityAction::Remove(pos))
                    .unwrap();
            }
            if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                self.block_entity_actions
                    .0
                    .send(BlockEntityAction::Create(pos))
                    .unwrap();
            }
            true
        } else {
            false
        }
    }

    pub fn update_block(&self, pos: Position) {
        // Before the flatterning, the client was expected to make changes to
        // blocks itself. For example, with doors, the server would only send
        // updates for one half when it was opened or closed, and the client
        // was responsible for updating the other half locally. After the
        // flatterning, the server sends updates for both halves of the door,
        // so we don't need to update the block around it locally.
        if self.protocol_version < 404 {
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
    }

    fn update_range(&self, x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) {
        // Before the flatterning, the client was expected to make changes to
        // blocks itself. For example, with doors, the server would only send
        // updates for one half when it was opened or closed, and the client
        // was responsible for updating the other half locally. After the
        // flatterning, the server sends updates for both halves of the door,
        // so we don't need to update the block around it locally.
        if self.protocol_version < 404 {
            for by in y1..y2 {
                for bz in z1..z2 {
                    for bx in x1..x2 {
                        let bp = Position::new(bx, by, bz);
                        let current = self.get_block(bp);
                        let new = current.update_state(self, bp);
                        if current != new {
                            self.set_block_raw(bp, new);
                        }
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
    }

    pub fn get_block(&self, pos: Position) -> block::Block {
        match self.chunks.read().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_block(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => block::Missing {},
        }
    }

    pub(crate) fn set_block_light(&self, pos: Position, light: u8) {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let mut chunks = self.chunks.write();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        chunk.set_block_light(pos.x & 0xF, pos.y, pos.z & 0xF, light);
    }

    pub fn get_block_light(&self, pos: Position) -> u8 {
        match self.chunks.read().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_block_light(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => 0,
        }
    }

    pub(crate) fn set_sky_light(&self, pos: Position, light: u8) {
        let cpos = CPos(pos.x >> 4, pos.z >> 4);
        let mut chunks = self.chunks.write();
        let chunk = chunks.entry(cpos).or_insert_with(|| Chunk::new(cpos));
        chunk.set_sky_light(pos.x & 0xF, pos.y, pos.z & 0xF, light);
    }

    pub fn get_sky_light(&self, pos: Position) -> u8 {
        match self.chunks.read().get(&CPos(pos.x >> 4, pos.z >> 4)) {
            Some(chunk) => chunk.get_sky_light(pos.x & 0xF, pos.y, pos.z & 0xF),
            None => 15,
        }
    }

    fn update_light(&self, pos: Position, ty: LightType) {
        self.light_updates.send(LightUpdate { ty, pos }).unwrap();
    }

    pub fn add_block_entity_action(&self, action: BlockEntityAction) {
        self.block_entity_actions.0.send(action).unwrap();
    }

    #[allow(clippy::verbose_bit_mask)] // "llvm generates better code" for updates_performed & 0xFFF "on x86"
    pub fn tick(&self, m: &mut ecs::Manager) {
        while let Ok(action) = self.block_entity_actions.1.try_recv() {
            match action {
                BlockEntityAction::Remove(pos) => {
                    if let Some(chunk) = self.chunks.write().get_mut(&CPos(pos.x >> 4, pos.z >> 4))
                    {
                        if let Some(entity) = chunk.block_entities.remove(&pos) {
                            m.world.despawn(entity);
                        }
                    }
                }
                BlockEntityAction::Create(pos) => {
                    if let Some(chunk) = self.chunks.write().get_mut(&CPos(pos.x >> 4, pos.z >> 4))
                    {
                        // Remove existing entity
                        if let Some(entity) = chunk.block_entities.remove(&pos) {
                            m.world.despawn(entity);
                        }
                        let block = chunk.get_block(pos.x & 0xF, pos.y, pos.z & 0xF);
                        if let Some(entity_type) =
                            block_entity::BlockEntityType::get_block_entity(block)
                        {
                            let entity = entity_type.create_entity(m, pos);
                            chunk.block_entities.insert(pos, entity);
                        }
                    }
                }
                BlockEntityAction::UpdateSignText(bx) => {
                    let (pos, line1, line2, line3, line4) = *bx;
                    if let Some(chunk) = self.chunks.write().get(&CPos(pos.x >> 4, pos.z >> 4)) {
                        if let Some(entity) = chunk.block_entities.get(&pos) {
                            if let Some(mut sign) = m
                                .world
                                .get_entity_mut(*entity)
                                .unwrap()
                                .get_mut::<SignInfo>()
                            {
                                sign.lines = [line1, line2, line3, line4];
                                sign.dirty = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // TODO: make use of "do_light_update"
    #[allow(dead_code)]
    pub(crate) fn do_light_update(&self, update: LightUpdate) {
        use std::cmp;
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

    pub fn copy_cloud_heightmap(&self, data: &mut [u8]) -> bool {
        let mut dirty = false;
        for c in self.chunks.write().values_mut() {
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

    pub fn get_dirty_chunk_sections(&self) -> Vec<(i32, i32, i32)> {
        let mut out = vec![];
        for chunk in self.chunks.read().values() {
            for sec in &chunk.sections {
                if let Some(sec) = sec.as_ref() {
                    if !sec.building && sec.dirty {
                        out.push((chunk.position.0, sec.y as i32, chunk.position.1));
                    }
                }
            }
        }
        out
    }

    fn set_dirty(&self, x: i32, y: i32, z: i32) {
        if let Some(chunk) = self.chunks.write().get_mut(&CPos(x, z)) {
            if let Some(sec) = chunk.sections.get_mut(y as usize).and_then(|v| v.as_mut()) {
                sec.dirty = true;
            }
        }
    }

    pub fn is_section_dirty(&self, pos: (i32, i32, i32)) -> bool {
        if let Some(chunk) = self.chunks.read().get(&CPos(pos.0, pos.2)) {
            if let Some(sec) = chunk.sections[pos.1 as usize].as_ref() {
                return sec.dirty && !sec.building;
            }
        }
        false
    }

    pub fn set_building_flag(&self, pos: (i32, i32, i32)) {
        if let Some(chunk) = self.chunks.write().get_mut(&CPos(pos.0, pos.2)) {
            if let Some(sec) = chunk.sections[pos.1 as usize].as_mut() {
                sec.building = true;
                sec.dirty = false;
            }
        }
    }

    pub fn reset_building_flag(&self, pos: (i32, i32, i32)) {
        if let Some(chunk) = self.chunks.write().get_mut(&CPos(pos.0, pos.2)) {
            if let Some(section) = chunk.sections[pos.1 as usize].as_mut() {
                section.building = false;
            }
        }
    }

    pub fn flag_dirty_all(&self) {
        for chunk in self.chunks.write().values_mut() {
            for sec in &mut chunk.sections {
                if let Some(sec) = sec.as_mut() {
                    sec.dirty = true;
                }
            }
        }
    }

    pub fn capture_snapshot(&self, x: i32, y: i32, z: i32) -> Option<ChunkSectionSnapshot> {
        // TODO: Improve performance!
        let cx = x >> 4;
        let cy = y >> 4;
        let cz = z >> 4;
        let chunks = self.chunks.read();
        let chunk = match chunks.get(&CPos(cx, cz)) {
            Some(val) => val,
            None => {
                return None;
            }
        };
        let sec = &chunk.sections[cy as usize];
        if sec.is_none() {
            return None;
        }
        return Some(sec.as_ref().unwrap().capture_snapshot(chunk.biomes));
    }

    pub fn unload_chunk(&self, x: i32, z: i32, m: &mut ecs::Manager) {
        if let Some(chunk) = self.chunks.write().remove(&CPos(x, z)) {
            for entity in chunk.block_entities.values() {
                m.world.despawn(*entity);
            }
        }
    }

    pub fn load_chunk(
        &self,
        x: i32,
        z: i32,
        new: bool,
        skylight: bool,
        read_biomes: bool,
        mask: u16,
        mask_add: u16,
        data: &mut Cursor<Vec<u8>>,
        version: u8,
    ) -> Result<(), protocol::Error> {
        let additional_light_data = self.lighting_cache.clone().write().remove(&CPos(x, z));
        let has_add_light = additional_light_data.is_some();
        let cpos = CPos(x, z);
        {
            let mut chunk = if new {
                // TODO: Improve lighting with something similar to bixilon's light accessor!
                Chunk::new(cpos)
            } else {
                match self.chunks.read().get(&cpos) {
                    Some(chunk) => chunk.clone(),
                    None => return Ok(()),
                }
            };

            // Block type array - whole byte per block  // 17
            let mut block_types: [[u8; 4096]; 16] = [[0u8; 4096]; 16]; // 17
            for (i, block_type) in block_types.iter_mut().enumerate() {
                if chunk.sections[i].is_none() {
                    let mut fill_sky = chunk.sections.iter().skip(i).all(|v| v.is_none());
                    fill_sky &= (mask & !((1 << i) | ((1 << i) - 1))) == 0;
                    fill_sky &= self.dimension.load().has_sky_light();
                    if !fill_sky || mask & (1 << i) != 0 {
                        chunk.sections[i] = Some(ChunkSection::new(i as u8, fill_sky));
                    }
                }
                if mask & (1 << i) == 0 {
                    continue;
                }

                if version == 17 {
                    data.read_exact(block_type)?;
                } else if version == 18 {
                    self.prep_section_18(&mut chunk, data, i);
                } else if version == 19 {
                    self.prep_section_19(&mut chunk, data, i, skylight);
                }
                let section = chunk.sections[i].as_mut().unwrap();
                section.dirty = true;
            }
            if version == 17 {
                self.finish_17(&mut chunk, mask, mask_add, skylight, data, block_types);
            } else if version != 19 {
                self.read_light(&mut chunk, mask, skylight, data);
            } else if has_add_light {
                let mut additional_light_data = additional_light_data.unwrap();
                self.load_light(
                    &mut chunk,
                    additional_light_data.block_light_mask,
                    true,
                    additional_light_data.sky_light_mask,
                    &mut additional_light_data.arrays,
                );
            }

            if new && read_biomes {
                // read biomes is always true (as param) except for load_chunk_19
                data.read_exact(&mut chunk.biomes)?;
            }

            chunk.calculate_heightmap();

            self.chunks.write().insert(cpos, chunk);
        }

        self.dirty_chunks_by_bitmask(x, z, mask);
        Ok(())
    }

    fn prep_section_19(
        &self,
        chunk: &mut Chunk,
        data: &mut Cursor<Vec<u8>>,
        section_id: usize,
        skylight: bool,
    ) {
        use crate::protocol::LenPrefixed;
        use leafish_protocol::types::bit;
        if self.protocol_version >= 451 {
            let _block_count = data.read_u16::<byteorder::LittleEndian>().unwrap();
            // TODO: use block_count
        }
        let section = chunk.sections[section_id].as_mut().unwrap();

        let mut bit_size = data.read_u8().unwrap();
        let mut mappings: BTreeMap<usize, block::Block> = BTreeMap::new();

        if bit_size == 0 {
            bit_size = 13;
        } else if bit_size < 4 {
            bit_size = 4;
        }

        if bit_size <= 8 {
            let count = VarInt::read_from(data).unwrap().0;
            for i in 0..count {
                let id = VarInt::read_from(data).unwrap().0;
                let bl = self
                    .id_map
                    .by_vanilla_id(id as usize, self.modded_block_ids.load().as_ref());
                mappings.insert(i as usize, bl);
            }
        }

        let bits = LenPrefixed::<VarInt, u64>::read_from(data).unwrap().data;
        let padded = self.protocol_version >= 736;
        let m = bit::Map::from_raw(bits, bit_size as usize, padded);

        for block_index in 0..4096 {
            let id = m.get(block_index);
            section.blocks_mut().set(
                block_index,
                mappings
                    .get(&id)
                    .cloned()
                    // TODO: fix or_fun_call, but do not re-borrow self
                    .unwrap_or_else(|| {
                        self.id_map
                            .by_vanilla_id(id, self.modded_block_ids.load().as_ref())
                    }),
            );
            // Spawn block entities
            let b = section.blocks_mut().get(block_index);
            if block_entity::BlockEntityType::get_block_entity(b).is_some() {
                let pos = Position::new(
                    (block_index & 0xF) as i32,
                    (block_index >> 8) as i32,
                    ((block_index >> 4) & 0xF) as i32,
                ) + (
                    chunk.position.0 << 4,
                    (section_id << 4) as i32,
                    chunk.position.1 << 4,
                );
                if chunk.block_entities.contains_key(&pos) {
                    self.block_entity_actions
                        .0
                        .send(BlockEntityAction::Remove(pos))
                        .unwrap();
                }
                self.block_entity_actions
                    .0
                    .send(BlockEntityAction::Create(pos))
                    .unwrap();
            }
        }
        if self.protocol_version >= 451 {
            // Skylight in update skylight packet for 1.14+
        } else {
            data.read_exact(&mut section.block_light.data).unwrap();
            if skylight {
                data.read_exact(&mut section.sky_light.data).unwrap();
            }
        }
    }

    fn prep_section_18(&self, chunk: &mut Chunk, data: &mut Cursor<Vec<u8>>, section_id: usize) {
        let section = chunk.sections[section_id].as_mut().unwrap();
        for bi in 0..4096 {
            let id = data.read_u16::<byteorder::LittleEndian>().unwrap();
            section.blocks.set(
                bi,
                self.id_map
                    .by_vanilla_id(id as usize, &self.modded_block_ids.load()),
            );

            // Spawn block entities
            let b = section.blocks.get(bi);
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
                    self.block_entity_actions
                        .0
                        .send(BlockEntityAction::Remove(pos))
                        .unwrap();
                }
                self.block_entity_actions
                    .0
                    .send(BlockEntityAction::Create(pos))
                    .unwrap();
            }
        }
    }

    fn read_light(&self, chunk: &mut Chunk, mask: u16, skylight: bool, data: &mut Cursor<Vec<u8>>) {
        // Block light array - half byte per block
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let section = chunk.sections[i as usize].as_mut().unwrap();

            data.read_exact(&mut section.block_light.data).unwrap();
        }

        // Sky light array - half byte per block - only if 'skylight' is true
        if skylight {
            for i in 0..16 {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let section = chunk.sections[i as usize].as_mut().unwrap();

                data.read_exact(&mut section.sky_light.data).unwrap();
            }
        }
    }

    fn finish_17(
        &self,
        chunk: &mut Chunk,
        mask: u16,
        mask_add: u16,
        skylight: bool,
        data: &mut Cursor<Vec<u8>>,
        block_types: [[u8; 4096]; 16],
    ) {
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

        for (i, meta) in block_meta.iter_mut().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }

            data.read_exact(&mut meta.data).unwrap();
        }

        self.read_light(chunk, mask, skylight, data);

        // Add array - half byte per block - uses secondary bitmask
        let block_add: [nibble::Array; 16] = [
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

        for (i, meta) in block_meta.iter_mut().enumerate() {
            if mask_add & (1 << i) == 0 {
                continue;
            }
            data.read_exact(&mut meta.data).unwrap();
        }

        // Now that we have the block types, metadata, and add, combine to initialize the blocks
        for i in 0..16 {
            if mask & (1 << i) == 0 {
                continue;
            }

            let section = chunk.sections[i].as_mut().unwrap();

            for bi in 0..4096 {
                let id = ((block_add[i].get(bi) as u16) << 12)
                    | ((block_types[i][bi] as u16) << 4)
                    | (block_meta[i].get(bi) as u16);
                section.blocks.set(
                    bi,
                    self.id_map
                        .by_vanilla_id(id as usize, &self.modded_block_ids.load()),
                );

                // Spawn block entities
                let b = section.blocks.get(bi);
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
                        self.block_entity_actions
                            .0
                            .send(BlockEntityAction::Remove(pos))
                            .unwrap();
                    }
                    self.block_entity_actions
                        .0
                        .send(BlockEntityAction::Create(pos))
                        .unwrap();
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
                self.flag_section_dirty(x + pos.0, i + pos.1, z + pos.2);
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
        skylight: bool,
        mask: u16,
        data: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk(x, z, new, skylight, new, mask, 0, data, 18)
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

    // TODO: Fix weird outlier phantom(unreal) light sources showing up in the corners of 1.12 chunks!
    fn load_light(
        &self,
        chunk: &mut Chunk,
        block_light_mask: i64,
        sky_light: bool,
        sky_light_mask: i64,
        data: &mut Cursor<Vec<u8>>,
    ) {
        if sky_light {
            for i in 0..17 {
                if sky_light_mask & (1 << i) == 0 {
                    continue;
                }
                if i == 0 {
                    let _size = VarInt::read_from(data);

                    data.consume(2048);
                    continue;
                }
                let i = i - 1;
                if chunk.sections[i as usize].as_ref().is_none() {
                    chunk.sections[i as usize].replace(ChunkSection::new(i, false));
                }
                let section = chunk.sections[i as usize].as_mut().unwrap();
                let _size = VarInt::read_from(data);

                data.read_exact(&mut section.sky_light.data).unwrap();
            }
        }
        if sky_light_mask & (1 << 63) != 0 {
            let _size = VarInt::read_from(data);

            data.consume(2048);
        }
        for i in 0..17 {
            if block_light_mask & (1 << i) == 0 {
                continue;
            }
            if i == 0 {
                let _size = VarInt::read_from(data);

                data.consume(2048);
                continue;
            }
            let i = i - 1;
            if chunk.sections[i as usize].as_ref().is_none() {
                chunk.sections[i as usize].replace(ChunkSection::new(i, false));
            }
            let section = chunk.sections[i as usize].as_mut().unwrap();
            let _size = VarInt::read_from(data);

            data.read_exact(&mut section.block_light.data).unwrap();
        }
    }

    pub fn load_chunk19(
        &self,
        x: i32,
        z: i32,
        new: bool,
        sky_light: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk19_or_115(true, x, z, new, sky_light, mask, data)
    }

    pub fn load_chunk115(
        &self,
        x: i32,
        z: i32,
        new: bool,
        sky_light: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk19_or_115(false, x, z, new, sky_light, mask, data)
    }

    #[allow(clippy::or_fun_call)]
    fn load_chunk19_or_115(
        &self,
        read_biomes: bool,
        x: i32,
        z: i32,
        new: bool,
        sky_light: bool,
        mask: u16,
        data: Vec<u8>,
    ) -> Result<(), protocol::Error> {
        self.load_chunk(
            x,
            z,
            new,
            sky_light,
            read_biomes,
            mask,
            0,
            &mut Cursor::new(data),
            19,
        )
    }

    fn flag_section_dirty(&self, x: i32, y: i32, z: i32) {
        if !(0..=15).contains(&y) {
            return;
        }
        let cpos = CPos(x, z);
        if let Some(chunk) = self.chunks.write().get_mut(&cpos) {
            if let Some(sec) = chunk.sections[y as usize].as_mut() {
                sec.dirty = true;
            }
        }
    }

    pub fn set_dimension(&self, new_dimension: Dimension) {
        self.dimension.store(Arc::new(new_dimension));
    }
}

impl block::WorldAccess for World {
    fn get_block(&self, pos: Position) -> block::Block {
        World::get_block(self, pos)
    }
}

#[derive(Debug)]
pub enum DimensionID {
    Index(i32),
    Name(String),
    Tag(NamedTag),
}

#[derive(Default, Debug)]
pub enum Dimension {
    #[default]
    Overworld,
    Nether,
    End,
    Other(DimensionID),
}

impl Dimension {
    pub fn from_index(index: i32) -> Self {
        match index {
            -1 => Self::Nether,
            0 => Self::Overworld,
            1 => Self::End,
            _ => Self::Other(DimensionID::Index(index)),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "minecraft:the_nether" => Self::Nether,
            "minecraft:overworld" => Self::Overworld,
            "minecraft:the_end" => Self::End,
            _ => Self::Other(DimensionID::Name(name.to_string())),
        }
    }

    pub fn from_tag(tag: &NamedTag) -> Self {
        Self::Other(DimensionID::Tag(tag.clone()))
    }

    pub fn has_sky_light(&self) -> bool {
        matches!(*self, Dimension::Overworld)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_world(protocol_version: i32) -> World {
        let (tx, _) = unbounded();
        World::new(protocol_version, tx)
    }

    fn load_chunk(
        world: &World,
        x: i32,
        z: i32,
        new: bool,
        skylight: bool,
        read_biomes: bool,
        mask: u16,
        mask_add: u16,
        data: &[u8],
        version: u8,
    ) {
        let mut data = Cursor::new(data.to_vec());
        world
            .load_chunk(
                x,
                z,
                new,
                skylight,
                read_biomes,
                mask,
                mask_add,
                &mut data,
                version,
            )
            .unwrap();
    }

    #[test]
    fn parse_chunk_1_8_9() {
        let world = build_world(47);

        let data = include_bytes!("testdata/chunk_1.8.9.bin");
        load_chunk(&world, 0, 0, true, false, true, 0, 0, data, 18);

        let data = include_bytes!("testdata/chunk_1.8.9_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 511, 0, data, 18);
    }

    #[test]
    fn parse_chunk_1_9() {
        let world = build_world(107);

        let data = include_bytes!("testdata/chunk_1.9.bin");
        load_chunk(&world, 0, 0, true, true, true, 511, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.9_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 511, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_9_2() {
        let world = build_world(109);

        let data = include_bytes!("testdata/chunk_1.9.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.9.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 507, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_10_2() {
        let world = build_world(210);
        let data = include_bytes!("testdata/chunk_1.10.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 79, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.10.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 195, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_11() {
        let world = build_world(315);

        let data = include_bytes!("testdata/chunk_1.11.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.11_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 511, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_11_2() {
        let world = build_world(316);

        let data = include_bytes!("testdata/chunk_1.11.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 63, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.11.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 511, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_12_2() {
        let world = build_world(340);

        let data = include_bytes!("testdata/chunk_1.12.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.12.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 511, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_13_2() {
        let world = build_world(404);
        let data = include_bytes!("testdata/chunk_1.13.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.13.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_14() {
        let world = build_world(477);
        let data = include_bytes!("testdata/chunk_1.14.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.14_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 207, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_14_1() {
        let world = build_world(480);
        let data = include_bytes!("testdata/chunk_1.14.1.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.14.1_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_14_2() {
        let world = build_world(485);
        let data = include_bytes!("testdata/chunk_1.14.2.bin");
        load_chunk(&world, 0, 0, true, true, true, 15, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.14.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_14_3() {
        let world = build_world(490);
        let data = include_bytes!("testdata/chunk_1.14.3.bin");
        load_chunk(&world, 0, 0, true, true, true, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.14.3_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_14_4() {
        let world = build_world(498);
        let data = include_bytes!("testdata/chunk_1.14.4.bin");
        load_chunk(&world, 0, 0, true, true, true, 63, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.14.4_nether.bin");
        load_chunk(&world, 0, 0, true, false, true, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_15_1() {
        let world = build_world(575);
        let data = include_bytes!("testdata/chunk_1.15.1.bin");
        load_chunk(&world, 0, 0, true, true, false, 63, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.15.1_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_15_2() {
        let world = build_world(578);
        let data = include_bytes!("testdata/chunk_1.15.2.bin");
        load_chunk(&world, 0, 0, true, true, false, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.15.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 195, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_16() {
        let world = build_world(735);
        let data = include_bytes!("testdata/chunk_1.16.bin");
        load_chunk(&world, 0, 0, true, true, false, 15, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.16_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_16_1() {
        let world = build_world(736);
        let data = include_bytes!("testdata/chunk_1.16.1.bin");
        load_chunk(&world, 0, 0, true, true, false, 63, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.16.1_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 195, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_16_2() {
        let world = build_world(751);
        let data = include_bytes!("testdata/chunk_1.16.2.bin");
        load_chunk(&world, 0, 0, true, true, false, 15, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.16.2_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_16_3() {
        let world = build_world(753);
        let data = include_bytes!("testdata/chunk_1.16.3.bin");
        load_chunk(&world, 0, 0, true, true, false, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.16.3_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 255, 0, data, 19);
    }

    #[test]
    fn parse_chunk_1_16_4() {
        let world = build_world(754);
        let data = include_bytes!("testdata/chunk_1.16.4.bin");
        load_chunk(&world, 0, 0, true, true, false, 31, 0, data, 19);

        let data = include_bytes!("testdata/chunk_1.16.4_nether.bin");
        load_chunk(&world, 0, 0, true, false, false, 247, 0, data, 19);
    }
}
