use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use glam::{IVec2, UVec2, Vec3};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::block::{self, Block};

#[derive(Debug)]
pub struct Chunk {
    blocks: [[[Block; 16]; 16]; 16],
    start: Vec3,
}

impl Chunk {
    pub fn blocks(&self) -> &[[[Block; 16]; 16]; 16] {
        &self.blocks
    }

    pub fn start(&self) -> Vec3 {
        self.start
    }
}

pub struct Chunks {
    // keyed by index derived from x and z positions of its origin
    loaded: Arc<Mutex<HashMap<UVec2, Vec<Chunk>>>>,
    // used to track which are in progress so we don't load things twice.
    loading: Arc<Mutex<HashSet<UVec2>>>,
    // used by the scene to generate instances. cleared every update.
    // TODO: figure out how to do this
    //new_chunks: Arc<Mutex<HashSet<UVec2>>>,
    block_position: IVec2,
    chunk_position: IVec2,
    chunk_loader: Option<JoinHandle<()>>,
    loader_tx: Option<Sender<UVec2>>,
}

impl Drop for Chunks {
    fn drop(&mut self) {
        std::mem::drop(self.loader_tx.take());
        self.chunk_loader
            .take()
            .expect("valid chunk loader thread")
            .join()
            .expect("chunk loader joined cleanly");
    }
}

impl Chunks {
    pub fn new(seed: u32) -> Self {
        // TODO: shut these down correctly.
        let (loader_tx, loader_rx) = mpsc::channel();

        let terrain = Fbm::<Perlin>::new(seed)
            .set_frequency(1.0)
            .set_persistence(0.5)
            .set_lacunarity(2.208984375)
            .set_octaves(14);

        // Create a thread that will load chunks when requested.
        let loading = Arc::new(Mutex::new(HashSet::new()));
        let loading_clone = Arc::clone(&loading);

        // Create a thread that will store the loaded chunks when requested.
        let loaded = Arc::new(Mutex::new(HashMap::new()));
        let loaded_clone = Arc::clone(&loaded);

        let chunk_loader = thread::Builder::new()
            .name(String::from("chunk loader"))
            .spawn(move || {
                for key in loader_rx {
                    let chunks = load_chunks(&terrain, key);
                    println!("completed loading of chunk {key}");
                    loaded_clone
                        .lock()
                        .expect("locked loaded")
                        .insert(key, chunks);
                    loading_clone.lock().expect("loading locked").remove(&key);
                }
            })
            .expect("unable to create chunk loader thread");

        //let new_chunks = Arc::new(Mutex::new(HashSet::new()));
        //let new_chunks_clone = new_chunks.clone();

        Self {
            loaded,
            loading,
            //new_chunks,
            block_position: IVec2::ZERO,
            chunk_position: IVec2::ZERO,
            chunk_loader: Some(chunk_loader),
            loader_tx: Some(loader_tx),
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

    /*
    pub fn new_chunks(&self) -> Arc<Mutex<HashSet<UVec2>>> {
        self.new_chunks.clone()
    }
    */

    // returns true if new chunks were loaded or old ones were unloaded.
    pub fn update(&mut self, player_position: &Vec3) {
        // clamp to only positive positions.
        self.block_position = IVec2::new(
            max(player_position.x.floor() as i32, 0),
            max(player_position.z.floor() as i32, 0),
        );
        self.chunk_position = IVec2::new(self.block_position.x / 16, self.block_position.y / 16);

        let start_chunk_position = UVec2::new(
            max(0, self.chunk_position.x - 3) as u32,
            max(0, self.chunk_position.y - 3) as u32,
        );
        let end_chunk_position = UVec2::new(
            max(0, self.chunk_position.x + 3) as u32,
            max(0, self.chunk_position.y + 3) as u32,
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
                println!("asking to load {key}");
                loading.insert(key);
                self.loader_tx
                    .as_ref()
                    .unwrap()
                    .send(key)
                    .expect("send succeeded");
            }
        }
    }
}

fn load_chunks(terrain: &Fbm<Perlin>, key: UVec2) -> Vec<Chunk> {
    println!("loading chunk {key}");
    let mut chunks = Vec::new();
    for chunky in 0..8 {
        let mut chunk = Chunk {
            blocks: [[[Block::new(); 16]; 16]; 16],
            start: Vec3::new(
                16.0 * (key.x as f32),
                16.0 * (chunky as f32),
                16.0 * (key.y as f32),
            ),
        };
        for (x, row) in chunk.blocks.iter_mut().enumerate() {
            for (y, col) in row.iter_mut().enumerate() {
                for (z, block) in col.iter_mut().enumerate() {
                    let blockx = (x as u32) + (16 * key.x);
                    let blocky = (y as u32) + (16 * chunky);
                    let blockz = (z as u32) + (16 * key.y);
                    let point: [f64; 2] = [blockx as f64 / 256.0, blockz as f64 / 256.0];
                    let height = ((terrain.get(point) + 1.0) * 32.0) as f32;
                    if (blocky as f32) < 32.0 {
                        block.set_type(block::Type::Water);
                    }
                    if (blocky as f32) < height {
                        block.set_type(match blocky {
                            0..=35 => block::Type::Sand,
                            36..=48 => block::Type::Grass,
                            49..=55 => block::Type::Rock,
                            56.. => block::Type::Ice,
                        });
                    }
                }
            }
        }
        chunks.push(chunk);
    }
    chunks
}
