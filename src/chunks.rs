use crate::{
    block::{self, Block},
    terrain::{Biome, WorldTerrain, BEDROCK_LEVEL},
};
use glam::{IVec2, IVec3, UVec2, Vec3};
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

pub const WATER_LEVEL: f32 = 32.0;
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
        let point: [f64; 2] = [position.x as f64, position.z as f64];
        self.terrain.get(point).height as f32
    }

    pub fn set_block(&self, x: i32, y: i32, z: i32, block_type: block::Type) {
        let is_water = matches!(block_type, block::Type::Water);
        self.set_block_with_level(x, y, z, block_type, if is_water { 8 } else { 0 }, is_water)
    }

    pub fn set_block_with_level(
        &self,
        x: i32,
        y: i32,
        z: i32,
        block_type: block::Type,
        level: u8,
        is_source: bool,
    ) {
        // Prevent modifying blocks at or below bedrock level
        if x < 0 || y <= BEDROCK_LEVEL as i32 || z < 0 || y >= MAX_HEIGHT {
            return;
        }

        let chunk_x = (x as u32) / 16;
        let chunk_y = (y as usize) / 16;
        let chunk_z = (z as u32) / 16;

        let lx = (x as usize) % 16;
        let ly = (y as usize) % 16;
        let lz = (z as usize) % 16;

        let key = UVec2::new(chunk_x, chunk_z);
        if let Ok(mut loaded) = self.loaded.lock() {
            if let Some(col) = loaded.get_mut(&key) {
                if chunk_y < col.len() {
                    let block = &mut col[chunk_y].blocks[lx][lz][ly];
                    block.set_type(block_type);
                    block.set_level(level);
                    block.set_source(is_source);
                    col[chunk_y].increment_version();

                    let path = format!("worlds/{}/chunk_{}_{}.bin", self.world_name, key.x, key.y);
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

                        let nx = x + dx;
                        let ny = y + dy;
                        let nz = z + dz;

                        if nx < 0 || ny < 0 || nz < 0 || ny >= MAX_HEIGHT {
                            continue;
                        }

                        let ncx = (nx as u32) / 16;
                        let ncy = (ny as usize) / 16;
                        let ncz = (nz as u32) / 16;

                        if ncx != chunk_x || ncy != chunk_y || ncz != chunk_z {
                            let n_key = UVec2::new(ncx, ncz);
                            if let Some(n_col) = loaded.get_mut(&n_key) {
                                if ncy < n_col.len() {
                                    let nlx = (nx as usize) % 16;
                                    let nly = (ny as usize) % 16;
                                    let nlz = (nz as usize) % 16;

                                    if n_col[ncy].blocks()[nlx][nlz][nly].is_active() {
                                        n_col[ncy].increment_version();
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
            if matches!(block_type, block::Type::Water) {
                queue.insert(IVec3::new(x, y, z));
            } else {
                // If a solid block is placed or block is destroyed, queue neighbors
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            queue.insert(IVec3::new(x + dx, y + dy, z + dz));
                        }
                    }
                }
            }
        }
    }

    pub fn is_solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        if let Ok(loaded) = self.loaded.lock() {
            if let Some(block) = get_block_at(&loaded, x, y, z) {
                return block.is_solid();
            }
        }
        false
    }

    pub fn block_material_at(&self, x: i32, y: i32, z: i32) -> u32 {
        if let Ok(loaded) = self.loaded.lock() {
            if let Some(block) = get_block_at(&loaded, x, y, z) {
                return block.material_id();
            }
        }
        0
    }

    pub fn is_chunk_loaded(&self, x: i32, z: i32) -> bool {
        let chunk_x = (x as u32) / 16;
        let chunk_z = (z as u32) / 16;
        let key = UVec2::new(chunk_x, chunk_z);
        if let Ok(loaded) = self.loaded.lock() {
            return loaded.contains_key(&key);
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
                let point: [f64; 2] = [blockx as f64, blockz as f64];
                let tdata = terrain.get(point);
                for (y, block) in col.iter_mut().enumerate() {
                    let blocky = (y as u32) + (16 * chunky);
                    let blockyf32 = blocky as f32;
                    let height = tdata.height as f32;

                    let point3d: [f64; 3] = [blockx as f64, blocky as f64, blockz as f64];

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

fn get_block_at(loaded: &HashMap<UVec2, Vec<Chunk>>, x: i32, y: i32, z: i32) -> Option<Block> {
    if x < 0 || y < 0 || z < 0 || y >= MAX_HEIGHT {
        return None;
    }
    let chunk_x = (x as u32) / 16;
    let chunk_y = (y as usize) / 16;
    let chunk_z = (z as u32) / 16;

    let lx = (x as usize) % 16;
    let ly = (y as usize) % 16;
    let lz = (z as usize) % 16;

    let key = UVec2::new(chunk_x, chunk_z);
    if let Some(col) = loaded.get(&key) {
        if chunk_y < col.len() {
            return Some(col[chunk_y].blocks()[lx][lz][ly]);
        }
    }
    None
}

fn set_block_in_sim(
    loaded: &mut HashMap<UVec2, Vec<Chunk>>,
    world_name: &str,
    pos: IVec3,
    block_type: block::Type,
    level: u8,
    is_source: bool,
    modified_chunks: &mut HashSet<UVec2>,
) {
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;

    if x < 0 || y <= BEDROCK_LEVEL as i32 || z < 0 || y >= MAX_HEIGHT {
        return;
    }

    let chunk_x = (x as u32) / 16;
    let chunk_y = (y as usize) / 16;
    let chunk_z = (z as u32) / 16;

    let lx = (x as usize) % 16;
    let ly = (y as usize) % 16;
    let lz = (z as usize) % 16;

    let key = UVec2::new(chunk_x, chunk_z);
    if let Some(col) = loaded.get_mut(&key) {
        if chunk_y < col.len() {
            let block = &mut col[chunk_y].blocks[lx][lz][ly];
            if block.ty() != block_type || block.level() != level || block.is_source() != is_source
            {
                block.set_type(block_type);
                block.set_level(level);
                block.set_source(is_source);
                col[chunk_y].increment_version();
                modified_chunks.insert(key);

                // Serialize chunk changes to disk
                let path = format!("worlds/{}/chunk_{}_{}.bin", world_name, key.x, key.y);
                if let Ok(data) = bincode::serialize(col) {
                    let _ = std::fs::write(&path, data);
                }
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

                let nx = x + dx;
                let ny = y + dy;
                let nz = z + dz;

                if nx < 0 || ny < 0 || nz < 0 || ny >= MAX_HEIGHT {
                    continue;
                }

                let ncx = (nx as u32) / 16;
                let ncy = (ny as usize) / 16;
                let ncz = (nz as u32) / 16;

                if ncx != chunk_x || ncy != chunk_y || ncz != chunk_z {
                    let n_key = UVec2::new(ncx, ncz);
                    if let Some(n_col) = loaded.get_mut(&n_key) {
                        if ncy < n_col.len() {
                            n_col[ncy].increment_version();
                            modified_chunks.insert(n_key);
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
        let x = pos.x;
        let y = pos.y;
        let z = pos.z;

        let current_block = match get_block_at(&loaded, x, y, z) {
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
            let block_above = get_block_at(&loaded, x, y + 1, z);
            let is_above_water = block_above.is_some_and(|b| matches!(b.ty(), block::Type::Water));

            if is_above_water {
                target_type = block::Type::Water;
                target_level = 8;
                target_is_source = false;
            } else {
                // Check horizontal neighbors for water.
                let mut max_neighbor_level = 0;
                let dirs = [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)];
                for &(dx, dy, dz) in dirs.iter() {
                    if let Some(b) = get_block_at(&loaded, x + dx, y + dy, z + dz) {
                        if matches!(b.ty(), block::Type::Water) && b.level() > max_neighbor_level {
                            max_neighbor_level = b.level();
                        }
                    }
                }

                if max_neighbor_level > 1 {
                    target_type = block::Type::Water;
                    target_level = max_neighbor_level - 1;
                    target_is_source = false;
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
                world_name,
                *pos,
                target_type,
                target_level,
                target_is_source,
                &mut modified_chunks,
            );

            // Queue self and neighbors for next tick
            next_queue.insert(*pos);
            let dirs = [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ];
            for &(dx, dy, dz) in dirs.iter() {
                next_queue.insert(IVec3::new(x + dx, y + dy, z + dz));
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
                    let world_x = x as f64;
                    let world_z = z as f64;
                    let height = terrain.get([world_x, world_z]).height;
                    for (y, block) in col.iter().enumerate() {
                        let blocky = (y as u32) + chunk_y_offset;

                        if (blocky as f64) < height - 10.0 {
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

        // Queue the water source position
        let start_pos = IVec3::new(5, 8, 5);
        water_queue.lock().unwrap().insert(start_pos);

        // Run tick 1: water should flow down to (5, 7, 5)
        let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
        tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);

        // Verify that (5, 7, 5) has become water with level 8
        {
            let l = loaded.lock().unwrap();
            let block_below = get_block_at(&l, 5, 7, 5).unwrap();
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

        // Verify that horizontal neighbors of (5, 7, 5) become water with level 7
        {
            let l = loaded.lock().unwrap();
            let north = get_block_at(&l, 6, 7, 5).unwrap();
            let south = get_block_at(&l, 4, 7, 5).unwrap();
            assert_eq!(north.ty(), block::Type::Water);
            assert_eq!(north.level(), 7);
            assert_eq!(south.ty(), block::Type::Water);
            assert_eq!(south.level(), 7);
        }

        // Now remove the source block at (5, 8, 5)
        {
            let mut l = loaded.lock().unwrap();
            let col = l.get_mut(&key).unwrap();
            col[0].blocks[5][5][8].set_type(block::Type::Inactive);
            col[0].blocks[5][5][8].set_level(0);
        }
        water_queue.lock().unwrap().insert(start_pos);

        // Tick several times: water should recede
        for _ in 0..10 {
            let mut jobs = std::mem::take(&mut *water_queue.lock().unwrap());
            tick_water_simulation("test_water", &loaded, &water_queue, &mut jobs);
        }

        // Verify that everything has dried up (become Inactive)
        {
            let l = loaded.lock().unwrap();
            let below = get_block_at(&l, 5, 7, 5).unwrap();
            let north = get_block_at(&l, 6, 7, 5).unwrap();
            assert_eq!(below.ty(), block::Type::Inactive);
            assert_eq!(north.ty(), block::Type::Inactive);
        }
    }
}
