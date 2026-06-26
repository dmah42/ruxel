use glam::{UVec2, Vec3};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::lsystem::{self, TreeType};
use crate::poisson::AdaptivePoisson;
use crate::terrain::{Biome, WorldTerrain, WATER_LEVEL};

const BUSH_MAX_HEIGHT: f64 = 50.0;

pub(crate) struct EntityManager {
    loaded_cells: HashMap<UVec2, lsystem::EntityMesh>,
    pub(crate) version: u32,
    task_tx: Sender<(u32, u32)>,
    result_rx: Receiver<(UVec2, lsystem::EntityMesh)>,
    in_flight: HashSet<UVec2>,
}

fn generate_entities_for_chunk(
    seed: u32,
    chunk_x: u32,
    chunk_z: u32,
    terrain: &WorldTerrain,
) -> Vec<(Vec3, TreeType)> {
    let mut entities = Vec::new();
    let poisson = AdaptivePoisson::new(seed);

    let points = poisson.generate_for_chunk(chunk_x, chunk_z, &|p| terrain.get(p));

    for pt in points {
        let (height, biome, grove) = terrain.get([pt.x as f64, pt.y as f64]);

        if height > WATER_LEVEL {
            let tree_type = match biome {
                Biome::Desert => TreeType::Palm,
                Biome::Plains => TreeType::Bush,
                Biome::Hills => {
                    if grove > 0.5 {
                        TreeType::Birch
                    } else {
                        TreeType::Oak
                    }
                }
                Biome::Mountains => {
                    let jitter = ((pt.x * 12.9898 + pt.y * 78.233).sin() * 10.0) as f64;
                    if height > 100.0 + jitter {
                        TreeType::Pine
                    } else {
                        TreeType::Birch
                    }
                }
                _ => TreeType::Default,
            };

            let position = Vec3::new(pt.x, height as f32, pt.y);
            entities.push((position, tree_type));
        }
    }

    // Pass 2: Bushes in Hills and Mountains
    let bush_poisson = AdaptivePoisson::new(seed.wrapping_add(1337));
    let bush_points = bush_poisson.generate_for_chunk(chunk_x, chunk_z, &|p| terrain.get(p));

    for pt in bush_points {
        let (height, biome, _) = terrain.get([pt.x as f64, pt.y as f64]);

        if height > WATER_LEVEL && height < BUSH_MAX_HEIGHT {
            let should_spawn = matches!(biome, Biome::Hills | Biome::Mountains);

            if should_spawn {
                let position = Vec3::new(pt.x, height as f32, pt.y);
                entities.push((position, TreeType::Bush));
            }
        }
    }

    entities
}

impl EntityManager {
    pub fn new(seed: u32, terrain: WorldTerrain) -> Self {
        let (task_tx, task_rx) = mpsc::channel::<(u32, u32)>();
        let (result_tx, result_rx) = mpsc::channel();

        thread::spawn(move || {
            while let Ok((chunk_x, chunk_z)) = task_rx.recv() {
                let mut mesh = lsystem::EntityMesh {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                };

                let entities = generate_entities_for_chunk(seed, chunk_x, chunk_z, &terrain);

                for (position, tree_type) in entities {
                    let tree_mesh = lsystem::generate_l_system_tree(tree_type, position);
                    let base_idx = mesh.vertices.len() as u32;
                    mesh.vertices.extend(tree_mesh.vertices);
                    mesh.indices
                        .extend(tree_mesh.indices.into_iter().map(|i| i + base_idx));
                }

                if result_tx
                    .send((UVec2::new(chunk_x, chunk_z), mesh))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            loaded_cells: HashMap::new(),
            version: 0,
            task_tx,
            result_rx,
            in_flight: HashSet::new(),
        }
    }

    pub fn loaded_cells(&self) -> &HashMap<UVec2, lsystem::EntityMesh> {
        &self.loaded_cells
    }

    fn queue_cell(&mut self, chunk_x: u32, chunk_z: u32) {
        let key = UVec2::new(chunk_x, chunk_z);
        if self.loaded_cells.contains_key(&key) || self.in_flight.contains(&key) {
            return; // Already loaded or in flight
        }

        self.in_flight.insert(key);
        let _ = self.task_tx.send((chunk_x, chunk_z));
    }

    pub(crate) fn update(&mut self, player_position: &Vec3, load_radius: u32) {
        // Process results
        while let Ok((key, mesh)) = self.result_rx.try_recv() {
            self.in_flight.remove(&key);

            let is_empty = mesh.vertices.is_empty();
            self.loaded_cells.insert(key, mesh);

            if !is_empty {
                // TODO: In the future, if trees can grow or meshes can change dynamically,
                // this check will need to be updated to detect structural changes,
                // not just whether the mesh is non-empty.
                self.version = self.version.wrapping_add(1);
            }
        }

        let block_x = std::cmp::max(player_position.x.floor() as i32, 0) as u32;
        let block_z = std::cmp::max(player_position.z.floor() as i32, 0) as u32;
        let chunk_x = block_x / 16;
        let chunk_z = block_z / 16;

        let start_x = chunk_x.saturating_sub(load_radius);
        let end_x = chunk_x.saturating_add(load_radius);
        let start_z = chunk_z.saturating_sub(load_radius);
        let end_z = chunk_z.saturating_add(load_radius);

        // Simple square load around player
        for cx in start_x..=end_x {
            for cz in start_z..=end_z {
                self.queue_cell(cx, cz);
            }
        }

        // Unload chunks outside radius
        let mut to_remove = Vec::new();
        for key in self.loaded_cells.keys().chain(self.in_flight.iter()) {
            if key.x < start_x || key.x > end_x || key.y < start_z || key.y > end_z {
                to_remove.push(*key);
            }
        }

        let mut removed_any = false;
        for key in to_remove {
            if self.loaded_cells.remove(&key).is_some() {
                removed_any = true;
            }
            self.in_flight.remove(&key);
        }

        if removed_any {
            self.version = self.version.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::WorldTerrain;
    use std::time::Duration;

    #[test]
    fn test_entity_manager_caching_empty_chunk() {
        let terrain = WorldTerrain::new(12345);
        let mut em = EntityManager::new(12345, terrain.clone());

        let player_pos = Vec3::new(0.0, 0.0, 0.0);

        // Initial update queues the chunk
        em.update(&player_pos, 0);

        // Wait for background thread to generate the mesh
        thread::sleep(Duration::from_millis(100));

        // Second update processes the result and caches it
        em.update(&player_pos, 0);

        let key = UVec2::new(0, 0);
        assert!(
            em.loaded_cells.contains_key(&key),
            "Chunk should be cached even if empty"
        );
    }

    #[test]
    fn test_entity_manager_version_bump() {
        let terrain = WorldTerrain::new(12345);
        let mut em = EntityManager::new(12345, terrain.clone());

        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        em.update(&player_pos, 0);

        thread::sleep(Duration::from_millis(100));
        em.update(&player_pos, 0);

        let key = UVec2::new(0, 0);
        let mesh = em.loaded_cells.get(&key).unwrap();

        if mesh.vertices.is_empty() {
            assert_eq!(em.version, 0, "Version should not bump for empty chunks");
        } else {
            assert_eq!(em.version, 1, "Version should bump for populated chunks");
        }
    }

    #[test]
    fn test_entity_generation_bitmap() {
        let terrain = WorldTerrain::new(12345);
        let width = 10000;
        let height = 10000;
        let mut img = image::ImageBuffer::new(width, height);

        // Fill background with heightmap
        for x in 0..width {
            for z in 0..height {
                let (h, _biome, grove_noise) = terrain.get([x as f64, z as f64]);
                if h <= crate::terrain::WATER_LEVEL {
                    let depth = (crate::terrain::WATER_LEVEL - h).clamp(0.0, 30.0);
                    let b = 255 - (depth as u8 * 4);
                    img.put_pixel(x, z, image::Rgb([0, 0, b]));
                } else {
                    let intensity =
                        ((h - crate::terrain::WATER_LEVEL) / 80.0 * 255.0).clamp(0.0, 255.0) as u8;
                    if grove_noise > 0.5 {
                        img.put_pixel(x, z, image::Rgb([intensity / 2, intensity, intensity / 2]));
                    } else {
                        img.put_pixel(
                            x,
                            z,
                            image::Rgb([
                                intensity / 2 + 30,
                                intensity / 2 + 30,
                                intensity / 2 + 30,
                            ]),
                        );
                    }
                }
            }
        }

        let chunk_count_w = width / 16;
        let chunk_count_h = height / 16;
        for cx in 0..chunk_count_w {
            for cz in 0..chunk_count_h {
                let entities = generate_entities_for_chunk(12345, cx, cz, &terrain);

                for (p, tree_type) in entities {
                    let px = p.x.round() as u32;
                    let pz = p.z.round() as u32;

                    if px < width && pz < height {
                        let color: [u8; 3] = match tree_type {
                            TreeType::Bush => [85, 107, 47],    // Dark Olive
                            TreeType::Palm => [255, 255, 0],    // Yellow
                            TreeType::Pine => [200, 200, 200],  // Grey/White
                            TreeType::Birch => [255, 255, 255], // White
                            TreeType::Oak => [100, 255, 100],   // Light green
                            TreeType::Default => [255, 0, 0],   // Red
                        };

                        for dx in -1..=1 {
                            for dz in -1..=1 {
                                let dx_px = (px as i32 + dx).clamp(0, width as i32 - 1) as u32;
                                let dz_pz = (pz as i32 + dz).clamp(0, height as i32 - 1) as u32;
                                img.put_pixel(dx_px, dz_pz, image::Rgb(color));
                            }
                        }
                    }
                }
            }
        }

        img.save("test_outputs/entity_generation_map.bmp")
            .expect("Failed to save entity generation map");
    }
}
