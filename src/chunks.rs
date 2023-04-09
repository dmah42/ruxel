use std::{cmp::max, collections::HashMap};

use glam::Vec3;
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
    loaded: HashMap<(u32, u32), Vec<Chunk>>,
    terrain: Box<dyn NoiseFn<f64, 2>>,
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
        }
    }

    pub fn loaded(&self) -> &HashMap<(u32, u32), Vec<Chunk>> {
        &self.loaded
    }

    // returns true if new chunks were loaded or old ones were unloaded.
    pub fn update(&mut self, player_position: &glam::Vec3) -> bool {
        let mut loaded = false;
        // clamp to only positive positions.
        let block_position = (
            max(player_position.x.floor() as i32, 0),
            max(player_position.z.floor() as i32, 0),
        );
        let chunk_position = (block_position.0 % 16, block_position.1 % 16);

        // TODO: expand this beyond the chunks immediately around the player.
        let start_chunk_position = (
            max(0, chunk_position.0 - 1) as u32,
            max(0, chunk_position.1 - 1) as u32,
        );
        let end_chunk_position = (
            max(0, chunk_position.0 + 1) as u32,
            max(0, chunk_position.1 + 1) as u32,
        );

        //let before_clean = self.loaded.len();
        //self.loaded.retain(|chunk, _| {
        //    chunk.0 >= start_chunk_position.0
        //        && chunk.1 >= start_chunk_position.1
        //        && chunk.0 <= end_chunk_position.0
        //        && chunk.1 <= end_chunk_position.1
        //});
        //if self.loaded.len() != before_clean {
        //    println!("unloaded {} chunks", before_clean - self.loaded.len());
        //}

        // TODO: stick it in a thread
        for chunkx in start_chunk_position.0..=end_chunk_position.0 {
            for chunkz in start_chunk_position.1..=end_chunk_position.1 {
                if self.loaded.contains_key(&(chunkx, chunkz)) {
                    //println!("skipping already loaded chunk ({chunkx}, {chunkz})");
                    continue;
                }
                loaded = true;
                println!("loading chunk ({chunkx}, {chunkz})");
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
                                    [blockx as f64 / 128.0, blockz as f64 / 128.0];
                                let height = ((self.terrain.get(point) + 1.0) * 64.0 / 2.0) as f32;
                                if (blocky as f32) < 32.0 {
                                    block.set_type(block::Type::Water);
                                }
                                if (blocky as f32) < height {
                                    block.set_type(match blocky {
                                        0..=35 => block::Type::Sand,
                                        36..=48 => block::Type::Grass,
                                        49..=55 => block::Type::Rock,
                                        56..=64 => block::Type::Ice,
                                        _ => panic!("unexpected height"),
                                    });
                                }
                            }
                        }
                    }
                    chunks.push(chunk);
                }
                self.loaded.insert((chunkx, chunkz), chunks);
            }
        }
        loaded
    }
}
