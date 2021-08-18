use crate::model;
use crate::render;
use crate::resources;
use crate::shared::Direction;
use crate::types::bit::Set;
use crate::world;
use crate::world::{block, World, SectionSnapshot, CPos};
use rand::{self, Rng, SeedableRng};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Instant;
// use rayon::prelude::*;

const NUM_WORKERS: usize = 8;

pub struct ChunkBuilder {
    threads: Vec<(mpsc::Sender<BuildReq>, thread::JoinHandle<()>)>,
    free_builders: Vec<(usize, Vec<u8>, Vec<u8>)>,
    built_recv: mpsc::Receiver<(usize, BuildReply)>,

    models: Arc<RwLock<model::Factory>>,
    resource_version: usize,
}

impl ChunkBuilder {

    pub fn new(
        resources: Arc<RwLock<resources::Manager>>,
        textures: Arc<RwLock<render::TextureManager>>,
    ) -> Self {
        let models = Arc::new(RwLock::new(model::Factory::new(resources, textures)));

        let mut threads = vec![];
        let mut free = vec![];
        let (built_send, built_recv) = mpsc::channel();
        for i in 0..NUM_WORKERS {
            let built_send = built_send.clone();
            let (work_send, work_recv) = mpsc::channel();
            let models = models.clone();
            let id = i;
            threads.push((
                work_send,
                thread::spawn(move || build_func_threaded(id, models, work_recv, built_send)),
            ));
            free.push((i, vec![], vec![]));
        }
        ChunkBuilder {
            threads,
            free_builders: free,
            built_recv,
            models,
            resource_version: 0xFFFF,
        }
    }

    pub fn tick(
        &mut self,
        world: Arc<World>,
        renderer: Arc<RwLock<render::Renderer>>,
        version: usize,
    ) {
        let now = Instant::now();
        if version != self.resource_version {
            self.resource_version = version;
            self.models.write().unwrap().version_change();
        }

        let renderer = renderer.clone();
        let mut renderer = renderer.write().unwrap();
            while let Ok((id, mut val)) = self.built_recv.try_recv() {
                world.clone().reset_building_flag(val.position);

                let world = world.clone();
                let chunks = world.chunks.clone();
                let chunk = chunks.get_mut(&CPos(val.position.0, val.position.2));
                if chunk.as_ref().is_some() {
                    let mut chunk = chunk.unwrap();
                    let section = if let Some(sec) = chunk.sections[val.position.1 as usize].as_mut() {
                        Some(sec)
                    } else {
                        None
                    };

                    if let Some(sec) = section {
                        sec.cull_info = val.cull_info;
                        renderer.update_chunk_solid(
                            sec.render_buffer.clone(),
                            &val.solid_buffer,
                            val.solid_count,
                        );
                        renderer.update_chunk_trans(
                            sec.render_buffer.clone(),
                            &val.trans_buffer,
                            val.trans_count,
                        );
                    }
                }

                val.solid_buffer.clear();
                val.trans_buffer.clear();
                self.free_builders
                    .push((id, val.solid_buffer, val.trans_buffer));
            }
        let diff = Instant::now().duration_since(now);
        println!("Diffchunk 1 took {}", diff.as_millis()); // readd
            if self.free_builders.is_empty() {
                return;
            }
        let tmp_world = world.clone();
        let dirty_sections = tmp_world // TODO: Improve perf!
            .get_render_list()
            .iter()//.par_iter()// .iter()//.par_iter_mut() // .par_iter()// .iter()
            .map(|v| v.0)
            .filter(|v| tmp_world.is_section_dirty(*v))
            .collect::<Vec<_>>();
        let diff = Instant::now().duration_since(now);
        println!("Diffchunk 2 took {}", diff.as_millis()); // readd
        for (x, y, z) in dirty_sections {
            tmp_world.set_building_flag((x, y, z));
            let t_id = self.free_builders.pop().unwrap();

                self.threads[t_id.0]
                    .0
                    .send(BuildReq {
                        world: world.clone(),
                        position: (x, y, z),
                        solid_buffer: t_id.1,
                        trans_buffer: t_id.2,
                    })
                    .unwrap();
                if self.free_builders.is_empty() {
                    return;
                }
        }
        let diff = Instant::now().duration_since(now);
        println!("Diffchunk 3 took {}", diff.as_millis()); // readd
    }

    pub fn reset(&mut self) {
        // TODO: Find a safer solution!
        // Drain built chunk data
        loop {
            let curr_data = self.built_recv.try_recv();
            if !curr_data.is_ok() {
                return;
            }
            let (id, mut val) = curr_data.unwrap();
            val.solid_buffer.clear();
            val.trans_buffer.clear();
            self.free_builders.push((id, val.solid_buffer, val.trans_buffer));
        }
    }

}

struct BuildReq {
    world: Arc<World>,
    position: (i32, i32, i32),
    solid_buffer: Vec<u8>,
    trans_buffer: Vec<u8>,
}

struct BuildReply {
    position: (i32, i32, i32),
    solid_buffer: Vec<u8>,
    solid_count: usize,
    trans_buffer: Vec<u8>,
    trans_count: usize,
    cull_info: CullInfo,
}

fn build_func_threaded(
    id: usize,
    models: Arc<RwLock<model::Factory>>,
    work_recv: mpsc::Receiver<BuildReq>,
    built_send: mpsc::Sender<(usize, BuildReply)>,
) {
    loop {
        let work: BuildReq = match work_recv.recv() {
            Ok(val) => val,
            Err(_) => return,
        };

        let reply = build_func_1(models.clone(), work);

        built_send.send((id, reply)).unwrap();
    }
}

fn build_func_1(models: Arc<RwLock<model::Factory>>, work: BuildReq) -> BuildReply {
    let BuildReq {
        world,
        position,
        mut solid_buffer,
        mut trans_buffer,
    } = work;
    let (cx, cy, cz) = (position.0 << 4, position.1 << 4, position.2 << 4);
    let mut snapshot = world.clone().capture_snapshot(cx, cy, cz);

    let mut rng = rand_pcg::Pcg32::from_seed([
        ((position.0 as u32) & 0xff) as u8,
        (((position.0 as u32) >> 8) & 0xff) as u8,
        (((position.0 as u32) >> 16) & 0xff) as u8,
        ((position.0 as u32) >> 24) as u8,
        ((position.1 as u32) & 0xff) as u8,
        (((position.1 as u32) >> 8) & 0xff) as u8,
        (((position.1 as u32) >> 16) & 0xff) as u8,
        ((position.1 as u32) >> 24) as u8,
        ((position.2 as u32) & 0xff) as u8,
        (((position.2 as u32) >> 8) & 0xff) as u8,
        (((position.2 as u32) >> 16) & 0xff) as u8,
        ((position.2 as u32) >> 24) as u8,
        (((position.0 as u32 ^ position.2 as u32) | 1) & 0xff) as u8,
        ((((position.0 as u32 ^ position.2 as u32) | 1) >> 8) & 0xff) as u8,
        ((((position.0 as u32 ^ position.2 as u32) | 1) >> 16) & 0xff) as u8,
        (((position.0 as u32 ^ position.2 as u32) | 1) >> 24) as u8,
    ]);

    let mut solid_count = 0;
    let mut trans_count = 0;

    match &snapshot {
        None => {
            // TODO: Handle this!
        },
        Some(snapshot) => {
            for y in 0..16 {
                for x in 0..16 {
                    for z in 0..16 {
                        let block = snapshot.get_block(x, y, z);
                        let mat = block.get_material();
                        if !mat.renderable {
                            // Use one step of the rng so that
                            // if a block is placed in an empty
                            // location is variant doesn't change
                            let _: u32 = rng.gen();
                            continue;
                        }

                        match block {
                            block::Block::Water { .. } | block::Block::FlowingWater { .. } => {
                                let tex = models.read().unwrap().textures.clone();
                                trans_count += model::liquid::render_liquid(
                                    tex,
                                    false,
                                    &snapshot,
                                    x,
                                    y,
                                    z,
                                    &mut trans_buffer,
                                );
                                continue;
                            }
                            block::Block::Lava { .. } | block::Block::FlowingLava { .. } => {
                                let tex = models.read().unwrap().textures.clone();
                                solid_count += model::liquid::render_liquid(
                                    tex,
                                    true,
                                    &snapshot,
                                    x,
                                    y,
                                    z,
                                    &mut solid_buffer,
                                );
                                continue;
                            }
                            _ => {}
                        }

                        if mat.transparent {
                            trans_count += model::Factory::get_state_model(
                                &models,
                                block,
                                &mut rng,
                                &snapshot,
                                x,
                                y,
                                z,
                                &mut trans_buffer,
                            );
                        } else {
                            solid_count += model::Factory::get_state_model(
                                &models,
                                block,
                                &mut rng,
                                &snapshot,
                                x,
                                y,
                                z,
                                &mut solid_buffer,
                            );
                        }
                    }
                }
            }
        }
    }

    let cull_info = build_cull_info(&snapshot.as_ref());

    BuildReply {
        position,
        solid_buffer,
        solid_count,
        trans_buffer,
        trans_count,
        cull_info,
    }
}

fn build_cull_info(snapshot: &Option<&world::SectionSnapshot>) -> CullInfo {
    if snapshot.is_none() {
        return CullInfo::all_vis();
    }
    let snapshot = snapshot.unwrap();
    let mut visited = Set::new(16 * 16 * 16);
    let mut info = CullInfo::new();

    for y in 0..16 {
        for z in 0..16 {
            for x in 0..16 {
                if visited.get(x | (z << 4) | (y << 8)) {
                    continue;
                }

                let touched = flood_fill(snapshot, &mut visited, x as i32, y as i32, z as i32);
                if touched == 0 {
                    continue;
                }

                for d1 in Direction::all() {
                    if (touched & (1 << d1.index())) != 0 {
                        for d2 in Direction::all() {
                            if (touched & (1 << d2.index())) != 0 {
                                info.set_visible(d1, d2);
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

fn flood_fill(snapshot: &world::SectionSnapshot, visited: &mut Set, x: i32, y: i32, z: i32) -> u8 {
    use std::collections::VecDeque;

    let mut next_position = VecDeque::with_capacity(16 * 16);
    next_position.push_back((x, y, z));

    let mut touched = 0;
    while let Some((x, y, z)) = next_position.pop_front() {
        let idx = (x | (z << 4) | (y << 8)) as usize;
        if !(0..=15).contains(&x)
            || !(0..=15).contains(&y)
            || !(0..=15).contains(&z)
            || visited.get(idx)
        {
            continue;
        }
        visited.set(idx, true);

        if snapshot
            .get_block(x, y, z)
            .get_material()
            .should_cull_against
        {
            continue;
        }

        if x == 0 {
            touched |= 1 << Direction::West.index();
        } else if x == 15 {
            touched |= 1 << Direction::East.index();
        }
        if y == 0 {
            touched |= 1 << Direction::Down.index();
        } else if y == 15 {
            touched |= 1 << Direction::Up.index();
        }
        if z == 0 {
            touched |= 1 << Direction::North.index();
        } else if z == 15 {
            touched |= 1 << Direction::South.index();
        }

        for d in Direction::all() {
            let (ox, oy, oz) = d.get_offset();
            next_position.push_back((x + ox, y + oy, z + oz));
        }
    }
    touched
}

#[derive(Clone, Copy, Default)]
pub struct CullInfo(u64);

impl CullInfo {
    pub fn new() -> CullInfo {
        Default::default()
    }

    pub fn all_vis() -> CullInfo {
        CullInfo(0xFFFFFFFFFFFFFFFF)
    }

    pub fn is_visible(&self, from: Direction, to: Direction) -> bool {
        (self.0 & (1 << (from.index() * 6 + to.index()))) != 0
    }

    pub fn set_visible(&mut self, from: Direction, to: Direction) {
        self.0 |= 1 << (from.index() * 6 + to.index());
    }
}
