use crate::{
    block::{self, Block},
    terrain::{Biome, WorldTerrain, BEDROCK_LEVEL, WATER_LEVEL},
};
use glam::{IVec2, IVec3, UVec2, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::{
        atomic,
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub const MAX_HEIGHT: i32 = 256;

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    // Stored as x, z, y (y is height)
    blocks: [[[Block; 16]; 16]; 16],
    start: Vec3,
    version: u32,
}

impl Chunk {
    pub fn blocks(&self) -> &[[[Block; 16]; 16]; 16] {
        &self.blocks
    }

    pub fn start(&self) -> Vec3 {
        self.start
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    #[cfg(test)]
    pub fn new(start: Vec3, blocks: [[[Block; 16]; 16]; 16]) -> Self {
        Self {
            blocks,
            start,
            version: 1,
        }
    }
}

pub struct Chunks {
    // keyed by index derived from x and z positions of its origin
    loaded: Arc<Mutex<HashMap<UVec2, Vec<Chunk>>>>,
    // used to track which are in progress so we don't load things twice.
    loading: Arc<Mutex<HashSet<UVec2>>>,
    block_position: IVec2,
    chunk_position: IVec2,
    chunk_loader: Option<JoinHandle<()>>,
    loader_tx: Option<Sender<UVec2>>,

    load_radius: i32,
    world_name: String,
    terrain: WorldTerrain,

    water_queue: Arc<Mutex<HashSet<IVec3>>>,
    sim_thread: Option<JoinHandle<()>>,
    sim_shutdown: Arc<atomic::AtomicBool>,
}

impl Drop for Chunks {
    fn drop(&mut self) {
        self.sim_shutdown.store(true, atomic::Ordering::Relaxed);
        if let Some(handle) = self.sim_thread.take() {
            let _ = handle.join();
        }

        std::mem::drop(self.loader_tx.take());
        self.chunk_loader
            .take()
            .expect("valid chunk loader thread")
            .join()
            .expect("chunk loader joined cleanly");
    }
}

struct LocalBlockCoords {
    chunk_key: UVec2,
    chunk_y: usize,
    lx: usize,
    ly: usize,
    lz: usize,
}

fn block_to_local_coords(pos: IVec3) -> Option<LocalBlockCoords> {
    if pos.x < 0 || pos.y < 0 || pos.z < 0 || pos.y >= MAX_HEIGHT {
        return None;
    }
    let chunk_x = (pos.x as u32) / 16;
    let chunk_y = (pos.y as usize) / 16;
    let chunk_z = (pos.z as u32) / 16;
    let lx = (pos.x as usize) % 16;
    let ly = (pos.y as usize) % 16;
    let lz = (pos.z as usize) % 16;
    Some(LocalBlockCoords {
        chunk_key: UVec2::new(chunk_x, chunk_z),
        chunk_y,
        lx,
        ly,
        lz,
    })
}

impl Chunks {
    pub fn new(world_name: String, seed: u32, load_radius: u32, sim_rate_ms: u64) -> Self {
        let _ = std::fs::create_dir_all(format!("worlds/{}", world_name));

        // TODO: shut these down correctly.
        let (loader_tx, loader_rx) = mpsc::channel();

        let terrain = WorldTerrain::new(seed);
        let terrain_clone = terrain.clone();

        // Create a thread that will load chunks when requested.
        let loading = Arc::new(Mutex::new(HashSet::new()));
        let loading_clone = Arc::clone(&loading);

        // Create a thread that will store the loaded chunks when requested.
        let loaded = Arc::new(Mutex::new(HashMap::new()));
        let loaded_clone = Arc::clone(&loaded);
        let world_name_clone = world_name.clone();

        let chunk_loader = thread::Builder::new()
            .name(String::from("chunk loader"))
            .spawn(move || {
                for key in loader_rx {
                    let chunks = load_chunks(&world_name_clone, &terrain_clone, key);
                    log::debug!("completed loading of chunk {key}");
                    let mut loaded = loaded_clone.lock().expect("locked loaded");
                    loaded.insert(key, chunks);

                    // Increment version of orthogonal loaded neighbors to force rebuild their boundaries
                    for dx in -1..=1 {
                        for dz in -1..=1 {
                            if (dx == 0) == (dz == 0) {
                                continue;
                            }
                            let nx = key.x as i32 + dx;
                            let nz = key.y as i32 + dz;
                            if nx >= 0 && nz >= 0 {
                                let n_key = glam::UVec2::new(nx as u32, nz as u32);
                                if let Some(n_col) = loaded.get_mut(&n_key) {
                                    for n_chunk in n_col.iter_mut() {
                                        n_chunk.increment_version();
                                    }
                                }
                            }
                        }
                    }

                    loading_clone.lock().expect("loading locked").remove(&key);
                }
            })
            .expect("unable to create chunk loader thread");

        let water_queue = Arc::new(Mutex::new(HashSet::new()));
        let water_queue_clone = Arc::clone(&water_queue);
        let loaded_clone_sim = Arc::clone(&loaded);
        let sim_shutdown = Arc::new(atomic::AtomicBool::new(false));
        let sim_shutdown_clone = Arc::clone(&sim_shutdown);
        let world_name_sim = world_name.clone();

        let sim_thread = thread::Builder::new()
            .name(String::from("water simulator"))
            .spawn(move || {
                while !sim_shutdown_clone.load(atomic::Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(sim_rate_ms));

                    let mut queue = {
                        let mut q = water_queue_clone.lock().expect("locked water queue");
                        if q.is_empty() {
                            continue;
                        }
                        std::mem::take(&mut *q)
                    };

                    tick_water_simulation(
                        &world_name_sim,
                        &loaded_clone_sim,
                        &water_queue_clone,
                        &mut queue,
                    );
                }
            })
            .expect("unable to create water simulation thread");

        Self {
            loaded,
            loading,
            block_position: IVec2::ZERO,
            chunk_position: IVec2::ZERO,
            chunk_loader: Some(chunk_loader),
            loader_tx: Some(loader_tx),

            load_radius: load_radius as i32,
            world_name,

            terrain,

            water_queue,
            sim_thread: Some(sim_thread),
            sim_shutdown,
        }
    }

    pub fn block_position(&self) -> &IVec2 {
        &self.block_position
    }

    pub fn chunk_position(&self) -> &IVec2 {
        &self.chunk_position
    }

    pub fn loaded(&self) -> Arc<Mutex<HashMap<UVec2, Vec<Chunk>>>> {
        Arc::clone(&self.loaded)
    }

    // returns true if new chunks were loaded or old ones were unloaded.
    pub fn update(&mut self, player_position: &Vec3) {
        // clamp to only positive positions.
        self.block_position = IVec2::new(
            max(player_position.x.floor() as i32, 0),
            max(player_position.z.floor() as i32, 0),
        );
        self.chunk_position = IVec2::new(self.block_position.x / 16, self.block_position.y / 16);

        let start_chunk_position = UVec2::new(
            max(0, self.chunk_position.x - self.load_radius) as u32,
            max(0, self.chunk_position.y - self.load_radius) as u32,
        );
        let end_chunk_position = UVec2::new(
            max(0, self.chunk_position.x + self.load_radius) as u32,
            max(0, self.chunk_position.y + self.load_radius) as u32,
        );

        // clean up any out of range chunks
        self.loaded
            .lock()
            .expect("lock loaded for retention")
            .retain(|chunk, _| {
                chunk.x >= start_chunk_position.x
                    && chunk.y >= start_chunk_position.y
                    && chunk.x <= end_chunk_position.x
                    && chunk.y <= end_chunk_position.y
            });

        for chunkx in start_chunk_position.x..=end_chunk_position.x {
            for chunkz in start_chunk_position.y..=end_chunk_position.y {
                let key = UVec2::new(chunkx, chunkz);
                let loaded = self.loaded.lock().expect("loaded locked");
                let mut loading = self.loading.lock().expect("loading locked");

                if loaded.contains_key(&key) || loading.contains(&key) {
                    continue;
                }
                log::debug!("asking to load {key}");
                loading.insert(key);
                self.loader_tx
                    .as_ref()
                    .unwrap()
                    .send(key)
                    .expect("send succeeded");
            }
        }
    }

    pub fn height_at(&self, position: &Vec3) -> f32 {
        let point = Vec2::new(position.x, position.z);
        self.terrain.get(point).height
    }

    pub fn set_block(&self, pos: glam::IVec3, block_type: block::Type) {
        let is_water = matches!(block_type, block::Type::Water);
        self.set_block_with_level(pos, block_type, if is_water { 8 } else { 0 }, is_water)
    }

    pub fn set_block_with_level(
        &self,
        pos: glam::IVec3,
        block_type: block::Type,
        level: u8,
        is_source: bool,
    ) {
        // Prevent modifying blocks at or below bedrock level
        if pos.y <= BEDROCK_LEVEL as i32 {
            return;
        }

        let coords = match block_to_local_coords(pos) {
            Some(c) => c,
            None => return,
        };

        if let Ok(mut loaded) = self.loaded.lock() {
            if let Some(col) = loaded.get_mut(&coords.chunk_key) {
                if coords.chunk_y < col.len() {
                    let block = &mut col[coords.chunk_y].blocks[coords.lx][coords.lz][coords.ly];
                    block.set_type(block_type);
                    block.set_level(level);
                    block.set_source(is_source);
                    col[coords.chunk_y].increment_version();

                    let path = format!(
                        "worlds/{}/chunk_{}_{}.bin",
                        self.world_name, coords.chunk_key.x, coords.chunk_key.y
                    );
                    if let Ok(data) = bincode::serialize(col) {
                        let _ = std::fs::write(&path, data);
                    }
                }
            }

            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if dx == 0 && dy == 0 && dz == 0 {
                            continue;
                        }

                        let npos = pos + IVec3::new(dx, dy, dz);
                        if let Some(nc) = block_to_local_coords(npos) {
                            if nc.chunk_key != coords.chunk_key || nc.chunk_y != coords.chunk_y {
                                if let Some(n_col) = loaded.get_mut(&nc.chunk_key) {
                                    if nc.chunk_y < n_col.len()
                                        && n_col[nc.chunk_y].blocks()[nc.lx][nc.lz][nc.ly]
                                            .is_active()
                                    {
                                        n_col[nc.chunk_y].increment_version();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Trigger queue updates for water simulation
        if let Ok(mut queue) = self.water_queue.lock() {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        queue.insert(pos + IVec3::new(dx, dy, dz));
                    }
                }
            }
        }
    }

    pub fn is_solid_at(&self, pos: glam::IVec3) -> bool {
        if let Ok(loaded) = self.loaded.lock() {
            if let Some(block) = get_block_at(&loaded, pos) {
                return block.is_solid();
            }
        }
        false
    }

    pub fn block_material_at(&self, pos: glam::IVec3) -> u32 {
        if let Ok(loaded) = self.loaded.lock() {
            if let Some(block) = get_block_at(&loaded, pos) {
                return block.material_id();
            }
        }
        0
    }

    pub fn is_chunk_loaded(&self, chunk_pos: glam::UVec2) -> bool {
        if let Ok(loaded) = self.loaded.lock() {
            return loaded.contains_key(&chunk_pos);
        }
        false
    }

    pub fn terrain(&self) -> &WorldTerrain {
        &self.terrain
    }
}

fn load_chunks(world_name: &str, terrain: &WorldTerrain, key: UVec2) -> Vec<Chunk> {
    log::debug!("loading chunk {key}");
    let path = format!("worlds/{}/chunk_{}_{}.bin", world_name, key.x, key.y);
    if let Ok(data) = std::fs::read(&path) {
        if let Ok(chunks) = bincode::deserialize(&data) {
            log::debug!("loaded chunk {} from disk", key);
            return chunks;
        }
    }

    let mut chunks = Vec::new();
    let num_chunks_y = (MAX_HEIGHT / 16) as u32;
    for chunky in 0..num_chunks_y {
        let mut chunk = Chunk {
            blocks: [[[Block::new(); 16]; 16]; 16],
            start: Vec3::new(
                16.0 * (key.x as f32),
                16.0 * (chunky as f32),
                16.0 * (key.y as f32),
            ),
            version: 1,
        };
        for (x, row) in chunk.blocks.iter_mut().enumerate() {
            for (z, col) in row.iter_mut().enumerate() {
                let blockx = (x as u32) + (16 * key.x);
                let blockz = (z as u32) + (16 * key.y);
                let point = glam::Vec2::new(blockx as f32, blockz as f32);
                let tdata = terrain.get(point);
                for (y, block) in col.iter_mut().enumerate() {
                    let blocky = (y as u32) + (16 * chunky);
                    let blockyf32 = blocky as f32;
                    let height = tdata.height;

                    let point3d = glam::Vec3::new(blockx as f32, blocky as f32, blockz as f32);

                    if terrain.is_cave(point3d, tdata.height) {
                        continue;
                    }

                    if blockyf32 < WATER_LEVEL && blockyf32 >= height {
                        block.set_type(block::Type::Water);
                    } else if blockyf32 < height {
                        let hash = (blockx.wrapping_mul(31)
                            ^ blocky.wrapping_mul(17)
                            ^ blockz.wrapping_mul(23))
                            % 10;
                        let dither = (hash as f32) - 5.0;

                        let btype = match tdata.biome {
                            Biome::Desert => {
                                if blockyf32 > height - 4.0 + (dither * 0.5) {
                                    block::Type::Sand
                                } else {
                                    block::Type::Rock
                                }
                            }
                            Biome::Ocean => {
                                if blockyf32 > height - 2.0 + (dither * 0.5) {
                                    block::Type::Sand
                                } else {
                                    block::Type::Rock
                                }
                            }
                            Biome::Plains | Biome::Hills => {
                                if blockyf32 > height - 1.0 {
                                    if height < WATER_LEVEL {
                                        block::Type::Sand
                                    } else {
                                        block::Type::Grass
                                    }
                                } else if blockyf32 > height - 4.0 + dither {
                                    block::Type::Sand
                                } else {
                                    block::Type::Rock
                                }
                            }
                            Biome::Mountains => {
                                if blockyf32 > 180.0 + dither {
                                    block::Type::Ice
                                } else if blockyf32 > 120.0 + dither {
                                    block::Type::Rock
                                } else if (blockyf32) > height - 1.0 {
                                    if height < WATER_LEVEL {
                                        block::Type::Sand
                                    } else {
                                        block::Type::Grass
                                    }
                                } else {
                                    block::Type::Rock
                                }
                            }
                        };
                        block.set_type(btype);
                    }
                }
            }
        }
        chunks.push(chunk);
    }
    chunks
}

fn get_block_at(loaded: &HashMap<UVec2, Vec<Chunk>>, pos: IVec3) -> Option<Block> {
    let coords = block_to_local_coords(pos)?;
    if let Some(col) = loaded.get(&coords.chunk_key) {
        if coords.chunk_y < col.len() {
            return Some(col[coords.chunk_y].blocks()[coords.lx][coords.lz][coords.ly]);
        }
    }
    None
}

fn set_block_in_sim(
    loaded: &mut HashMap<UVec2, Vec<Chunk>>,
    pos: IVec3,
    block_type: block::Type,
    level: u8,
    is_source: bool,
    modified_chunks: &mut HashSet<UVec2>,
) {
    if pos.y <= BEDROCK_LEVEL as i32 {
        return;
    }

    let coords = match block_to_local_coords(pos) {
        Some(c) => c,
        None => return,
    };

    if let Some(col) = loaded.get_mut(&coords.chunk_key) {
        if coords.chunk_y < col.len() {
            let block = &mut col[coords.chunk_y].blocks[coords.lx][coords.lz][coords.ly];
            if block.ty() != block_type || block.level() != level || block.is_source() != is_source
            {
                block.set_type(block_type);
                block.set_level(level);
                block.set_source(is_source);
                col[coords.chunk_y].increment_version();
                modified_chunks.insert(coords.chunk_key);
            }
        }
    }

    // Increment version of adjacent chunk columns if modifying boundary blocks
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }

                let npos = pos + IVec3::new(dx, dy, dz);
                if let Some(nc) = block_to_local_coords(npos) {
                    if nc.chunk_key != coords.chunk_key || nc.chunk_y != coords.chunk_y {
                        if let Some(n_col) = loaded.get_mut(&nc.chunk_key) {
                            if nc.chunk_y < n_col.len() {
                                n_col[nc.chunk_y].increment_version();
                                modified_chunks.insert(nc.chunk_key);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn tick_water_simulation(
    world_name: &str,
    loaded_lock: &Arc<Mutex<HashMap<UVec2, Vec<Chunk>>>>,
    water_queue: &Arc<Mutex<HashSet<IVec3>>>,
    queue_to_process: &mut HashSet<IVec3>,
) {
    let mut loaded = loaded_lock.lock().expect("locked loaded in sim");
    let mut modified_chunks = HashSet::new();
    let mut next_queue = HashSet::new();

    for pos in queue_to_process.iter() {
        let current_block = match get_block_at(&loaded, *pos) {
            Some(b) => b,
            None => continue, // Unloaded chunk
        };

        // We only simulate for Water or Inactive (empty/air) blocks
        if !matches!(
            current_block.ty(),
            block::Type::Inactive | block::Type::Water
        ) {
            continue;
        }

        // Determine target state based on neighbors
        let mut target_type = block::Type::Inactive;
        let mut target_level = 0;
        let mut target_is_source = false;

        if current_block.is_source() {
            target_type = block::Type::Water;
            target_level = current_block.level();
            target_is_source = true;
        } else {
            // Check above. If the block above is water, this block becomes falling water (level 8).
            let block_above = get_block_at(&loaded, *pos + IVec3::new(0, 1, 0));
            let is_above_water = block_above.is_some_and(|b| matches!(b.ty(), block::Type::Water));

            if is_above_water {
                target_type = block::Type::Water;
                target_level = 8;
                target_is_source = false;
            } else {
                // Check block below target
                let block_below_target = get_block_at(&loaded, *pos - IVec3::new(0, 1, 0));

                let (is_below_target_water, is_below_target_air) = match block_below_target {
                    Some(b) => match b.ty() {
                        block::Type::Water => {
                            if b.is_source() {
                                (true, false) // Ocean/source water: treat as water boundary
                            } else {
                                (false, true) // Falling water: treat as downward/air path
                            }
                        }
                        block::Type::Inactive => (false, true),
                        _ => (false, false), // Solid ground
                    },
                    None => (false, true), // Unloaded or empty: treat as air/downward path
                };

                if is_below_target_water {
                    // Do not allow horizontal spread over existing water sources (like oceans)
                    target_type = block::Type::Inactive;
                    target_level = 0;
                    target_is_source = false;
                } else {
                    // Check horizontal neighbors for water.
                    let mut max_neighbor_level = 0;
                    let dirs = [
                        IVec3::new(1, 0, 0),
                        IVec3::new(-1, 0, 0),
                        IVec3::new(0, 0, 1),
                        IVec3::new(0, 0, -1),
                    ];
                    for &dir in dirs.iter() {
                        let npos = *pos + dir;
                        if let Some(b) = get_block_at(&loaded, npos) {
                            if matches!(b.ty(), block::Type::Water) {
                                // Gravity-first check: only spread if neighbor is a source block OR is resting on a solid block
                                let can_spread = b.is_source() || {
                                    let below_neighbor = get_block_at(&loaded, npos - IVec3::new(0, 1, 0));
                                    below_neighbor.is_some_and(|below| below.is_solid())
                                };

                                if can_spread && b.level() > max_neighbor_level {
                                    max_neighbor_level = b.level();
                                }
                            }
                        }
                    }

                    // Determine target level
                    let computed_level = if is_below_target_air {
                        if max_neighbor_level >= 1 {
                            std::cmp::max(1, max_neighbor_level.saturating_sub(1))
                        } else {
                            0
                        }
                    } else {
                        max_neighbor_level.saturating_sub(1)
                    };

                    if computed_level >= 1 {
                        target_type = block::Type::Water;
                        target_level = computed_level;
                        target_is_source = false;
                    }
                }
            }
        }

        // Compare target state with current state
        if current_block.ty() != target_type
            || current_block.level() != target_level
            || current_block.is_source() != target_is_source
        {
            // Update the block
            set_block_in_sim(
                &mut loaded,
                *pos,
                target_type,
                target_level,
                target_is_source,
                &mut modified_chunks,
            );

            // Queue self and neighbors for next tick
            next_queue.insert(*pos);
            let dirs = [
                IVec3::new(1, 0, 0),
                IVec3::new(-1, 0, 0),
                IVec3::new(0, 1, 0),
                IVec3::new(0, -1, 0),
                IVec3::new(0, 0, 1),
                IVec3::new(0, 0, -1),
            ];
            for &dir in dirs.iter() {
                next_queue.insert(*pos + dir);
            }
        }
    }

    // Batch write modified chunks to disk before completing tick
    for key in modified_chunks.iter() {
        if let Some(col) = loaded.get(key) {
            let path = format!("worlds/{}/chunk_{}_{}.bin", world_name, key.x, key.y);
            if let Ok(data) = bincode::serialize(col) {
                let _ = std::fs::write(&path, data);
            }
        }
    }

    if !next_queue.is_empty() {
        let mut q = water_queue.lock().expect("locked water queue in sim");
        q.extend(next_queue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cave_generation_in_chunk() {
        let terrain = WorldTerrain::new(999);
        let key = UVec2::new(0, 0);
        let chunks = load_chunks("test_caves", &terrain, key);

        let mut solid_underground = 0;
        let mut cave_air = 0;

        for chunk in chunks.iter() {
            let chunk_y_offset = chunk.start.y as u32;
            for (x, row) in chunk.blocks.iter().enumerate() {
                for (z, col) in row.iter().enumerate() {
                    let world_x = x as f32;
                    let world_z = z as f32;
                    let height = terrain.get(Vec2::new(world_x, world_z)).height;
                    for (y, block) in col.iter().enumerate() {
                        let blocky = (y as u32) + chunk_y_offset;

                        if (blocky as f32) < height - 10.0f32 {
                            // deep underground
                            if block.is_active() {
                                solid_underground += 1;
                            } else {
                                cave_air += 1;
                            }
                        }
                    }
                }
            }
        }

        println!(
            "Solid blocks: {}, Cave air blocks: {}",
            solid_underground, cave_air
        );
        assert!(cave_air > 0, "No caves were generated in the test chunk!");
    }

    #[test]
    fn test_water_flow_simulation() {
        let loaded = Arc::new(Mutex::new(HashMap::new()));
        let water_queue = Arc::new(Mutex::new(HashSet::new()));

        // Create a 16x16x16 chunk at origin with start Vec3::new(0, 0, 0)
        let blocks = [[[Block::new(); 16]; 16]; 16];
        // Leave all blocks as Inactive (air)

        let chunk = Chunk::new(Vec3::new(0.0, 0.0, 0.0), blocks);
        let key = UVec2::new(0, 0);
        loaded.lock().unwrap().insert(key, vec![chunk]);

        // Place a water source block (level 8) at (5, 8, 5)
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[5][5][8].set_type(block::Type::Water);
            col[0].blocks[5][5][8].set_level(8);
            col[0].blocks[5][5][8].set_source(true);
        }

        // Queue the water source position and its neighbors
        {
            let mut q = water_queue.lock().unwrap();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        q.insert(IVec3::new(5 + dx, 8 + dy, 5 + dz));
                    }
                }
            }
        }

        // Run tick 1: water should flow down to (5, 7, 5)
        let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
        tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);

        // Verify that (5, 7, 5) has become water with level 8
        {
            let l = loaded.lock().unwrap();
            let block_below = get_block_at(&l, IVec3::new(5, 7, 5)).unwrap();
            assert_eq!(block_below.ty(), block::Type::Water);
            assert_eq!(block_below.level(), 8);
        }

        // Place a solid block at (5, 6, 5) to block downward flow
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[5][5][6].set_type(block::Type::Rock);
        }

        // Run tick 2: water is at (5, 7, 5). It hits solid rock at (5, 6, 5), so it should spread horizontally
        let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
        tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);

        // Verify that horizontal neighbors of (5, 7, 5) become water with level
        // 8 (due to falling water from Y=8 horizontal spread)
        {
            let l = loaded.lock().unwrap();
            let north = get_block_at(&l, IVec3::new(6, 7, 5)).unwrap();
            let south = get_block_at(&l, IVec3::new(4, 7, 5)).unwrap();
            assert_eq!(north.ty(), block::Type::Water);
            assert_eq!(north.level(), 8);
            assert_eq!(south.ty(), block::Type::Water);
            assert_eq!(south.level(), 8);
        }

        // Now remove the source block at (5, 8, 5)
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[5][5][8].set_type(block::Type::Inactive);
            col[0].blocks[5][5][8].set_level(0);
        }
        {
            let mut q = water_queue.lock().unwrap();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        q.insert(IVec3::new(5 + dx, 8 + dy, 5 + dz));
                    }
                }
            }
        }

        // Tick several times: water should recede
        for _ in 0..10 {
            let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
            tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);
        }
    }

    #[test]
    fn test_gravity_first_water_flow() {
        let loaded = Arc::new(Mutex::new(HashMap::new()));
        let water_queue = Arc::new(Mutex::new(HashSet::new()));

        // Create a 16x16x16 chunk at origin
        let mut blocks = [[[Block::new(); 16]; 16]; 16];

        // Create a platform of solid blocks at Y=5 (Z=5 in internal array)
        for x in 5..=10 {
            blocks[x][5][5].set_type(block::Type::Rock);
        }

        let chunk = Chunk::new(Vec3::new(0.0, 0.0, 0.0), blocks);
        let key = UVec2::new(0, 0);
        loaded.lock().unwrap().insert(key, vec![chunk]);

        // Place a water source block at (7, 6, 5) (x=7, z=5, y=6)
        // Its floor (7, 5, 5) is solid rock.
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[7][5][6].set_type(block::Type::Water);
            col[0].blocks[7][5][6].set_level(8);
            col[0].blocks[7][5][6].set_source(true);
        }

        // Queue the water source and its neighbors
        {
            let mut q = water_queue.lock().unwrap();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        q.insert(IVec3::new(7 + dx, 6 + dy, 5 + dz));
                    }
                }
            }
        }

        // Tick several times to let it spread
        for _ in 0..10 {
            let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
            tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);
        }

        // Verify:
        // 1. (7, 6, 5) is the source (Water, level 8)
        // 2. (8, 6, 5) is Water (level 7)
        // 3. (9, 6, 5) is Water (level 6)
        // 4. (10, 6, 5) is Water (level 5)
        // 5. (11, 6, 5) is above air (since platform ends at x=10). It should be Water (level 4).
        // 6. But it should NOT spread horizontally further to (12, 6, 5) in mid-air because the floor below (11, 5, 5) is air.
        {
            let l = loaded.lock().unwrap();

            // Source block
            let source = get_block_at(&l, IVec3::new(7, 6, 5)).unwrap();
            assert_eq!(source.ty(), block::Type::Water);

            // Flow on ground (x=10, y=6, z=5)
            let flow_on_ground = get_block_at(&l, IVec3::new(10, 6, 5)).unwrap();
            assert_eq!(flow_on_ground.ty(), block::Type::Water);

            // Flow just past the edge (x=11, y=6, z=5)
            let flow_at_edge = get_block_at(&l, IVec3::new(11, 6, 5)).unwrap();
            assert_eq!(flow_at_edge.ty(), block::Type::Water);
            assert_eq!(flow_at_edge.level(), 4);

            // Should NOT spread to x=12 in mid-air
            let past_edge = get_block_at(&l, IVec3::new(12, 6, 5)).unwrap();
            assert_eq!(past_edge.ty(), block::Type::Inactive);
        }
    }

    #[test]
    fn test_water_flow_into_hole() {
        let loaded = Arc::new(Mutex::new(HashMap::new()));
        let water_queue = Arc::new(Mutex::new(HashSet::new()));

        // Create a 16x16x16 chunk at origin
        let mut blocks = [[[Block::new(); 16]; 16]; 16];

        // Create a platform of solid blocks at Y=5 (Z=5 in internal array)
        // Platform exists for x in 5..=8. So x=9 is air/hole initially.
        for x in 5..=8 {
            blocks[x][5][5].set_type(block::Type::Rock);
        }

        let chunk = Chunk::new(Vec3::new(0.0, 0.0, 0.0), blocks);
        let key = UVec2::new(0, 0);
        loaded.lock().unwrap().insert(key, vec![chunk]);

        // Place a water source block at (7, 6, 5) with level 2 (source)
        // Its floor is solid.
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[7][5][6].set_type(block::Type::Water);
            col[0].blocks[7][5][6].set_level(2);
            col[0].blocks[7][5][6].set_source(true);
        }

        // Queue the water source and its neighbors
        {
            let mut q = water_queue.lock().unwrap();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        q.insert(IVec3::new(7 + dx, 6 + dy, 5 + dz));
                    }
                }
            }
        }

        // Tick 5 times to let it spread to (8, 6, 5), flow into (9, 6, 5), and fall to (9, 5, 5)
        for _ in 0..5 {
            let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
            tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);
        }

        // Verify that the water has flowed into the hole (9, 6, 5) and fallen down to (9, 5, 5)
        {
            let l = loaded.lock().unwrap();
            let flow_on_ground = get_block_at(&l, IVec3::new(8, 6, 5)).unwrap();
            assert_eq!(flow_on_ground.ty(), block::Type::Water);

            let flow_in_hole = get_block_at(&l, IVec3::new(9, 6, 5)).unwrap();
            assert_eq!(flow_in_hole.ty(), block::Type::Water);

            let falling_flow = get_block_at(&l, IVec3::new(9, 5, 5)).unwrap();
            assert_eq!(falling_flow.ty(), block::Type::Water);
            assert_eq!(falling_flow.level(), 8);
        }
    }
}
