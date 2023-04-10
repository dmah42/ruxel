use std::{cmp::max, collections::HashMap};

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
    loaded: HashMap<UVec2, Vec<Chunk>>,
    terrain: Box<dyn NoiseFn<f64, 2>>,
    block_position: IVec2,
    chunk_position: IVec2,
}

impl Chunks {
    pub fn new(seed: u32) -> Self {
        Self {
            loaded: HashMap::new(),
            terrain: Box::new(
                Fbm::<Perlin>::new(seed)
                    .set_frequency(1.0)
                    .set_persistence(0.5)
                    .set_lacunarity(2.208984375)
                    .set_octaves(14),
            ),
            block_position: IVec2::ZERO,
            chunk_position: IVec2::ZERO,
        }
    }

    pub fn block_position(&self) -> &IVec2 {
        &self.block_position
    }

    pub fn chunk_position(&self) -> &IVec2 {
        &self.chunk_position
    }

    pub fn loaded(&self) -> &HashMap<UVec2, Vec<Chunk>> {
        &self.loaded
    }

    // returns true if new chunks were loaded or old ones were unloaded.
    pub fn update(&mut self, player_position: &Vec3) -> bool {
        let mut loaded = false;
        // clamp to only positive positions.
        self.block_position = IVec2::new(
            max(player_position.x.floor() as i32, 0),
            max(player_position.z.floor() as i32, 0),
        );
        self.chunk_position = IVec2::new(self.block_position.x / 16, self.block_position.y / 16);

        let start_chunk_position = UVec2::new(
            max(0, self.chunk_position.x - 2) as u32,
            max(0, self.chunk_position.y - 2) as u32,
        );
        let end_chunk_position = UVec2::new(
            max(0, self.chunk_position.x + 2) as u32,
            max(0, self.chunk_position.y + 2) as u32,
        );

        // clean up any out of range chunks
        self.loaded.retain(|chunk, _| {
            chunk.x >= start_chunk_position.x
                && chunk.y >= start_chunk_position.y
                && chunk.x <= end_chunk_position.x
                && chunk.y <= end_chunk_position.y
        });

        // TODO: stick it in a thread
        for chunkx in start_chunk_position.x..=end_chunk_position.x {
            for chunkz in start_chunk_position.y..=end_chunk_position.y {
                let key = UVec2::new(chunkx, chunkz);
                if self.loaded.contains_key(&key) {
                    continue;
                }
                println!("loading chunk {key}");
                loaded = true;
                let mut chunks: Vec<Chunk> = vec![];
                for chunky in 0..8 {
                    let mut chunk = Chunk {
                        blocks: [[[Block::new(); 16]; 16]; 16],
                        start: Vec3::new(
                            16.0 * (chunkx as f32),
                            16.0 * (chunky as f32),
                            16.0 * (chunkz as f32),
                        ),
                    };
                    for (x, row) in chunk.blocks.iter_mut().enumerate() {
                        for (y, col) in row.iter_mut().enumerate() {
                            for (z, block) in col.iter_mut().enumerate() {
                                let blockx = (x as u32) + (16 * chunkx);
                                let blocky = (y as u32) + (16 * chunky);
                                let blockz = (z as u32) + (16 * chunkz);
                                let point: [f64; 2] =
                                    [blockx as f64 / 256.0, blockz as f64 / 256.0];
                                let height = ((self.terrain.get(point) + 1.0) * 32.0) as f32;
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
                self.loaded.insert(key, chunks);
            }
        }
        loaded
    }
}
