use glam::{UVec2, Vec3};
use std::collections::HashMap;

use crate::lsystem;
use crate::poisson::AdaptivePoisson;
use crate::terrain::{WorldTerrain, WATER_LEVEL};

pub(crate) struct EntityManager {
    poisson: AdaptivePoisson,
    loaded_cells: HashMap<UVec2, lsystem::EntityMesh>,
    pub(crate) version: u32,
}

impl EntityManager {
    pub fn new(seed: u32) -> Self {
        Self {
            poisson: AdaptivePoisson::new(seed),
            loaded_cells: HashMap::new(),
            version: 0,
        }
    }

    pub fn loaded_cells(&self) -> &HashMap<UVec2, lsystem::EntityMesh> {
        &self.loaded_cells
    }

    fn load_cell(&mut self, chunk_x: u32, chunk_z: u32, terrain: &WorldTerrain) {
        let key = UVec2::new(chunk_x, chunk_z);
        if self.loaded_cells.contains_key(&key) {
            return; // Already loaded
        }

        let mut mesh = lsystem::EntityMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        };

        let points = self
            .poisson
            .generate_for_chunk(chunk_x, chunk_z, &|p| terrain.get(p));

        for pt in points {
            let (height, _biome) = terrain.get([pt.x as f64, pt.y as f64]);

            // Only spawn trees on land
            if height > WATER_LEVEL {
                let position = Vec3::new(pt.x, height as f32, pt.y);
                let tree_mesh = lsystem::generate_l_system_tree(4, position);

                let base_idx = mesh.vertices.len() as u32;
                mesh.vertices.extend(tree_mesh.vertices);
                mesh.indices
                    .extend(tree_mesh.indices.into_iter().map(|i| i + base_idx));
            }
        }

        if !mesh.vertices.is_empty() {
            self.loaded_cells.insert(key, mesh);
            self.version = self.version.wrapping_add(1);
        }
    }

    pub(crate) fn update(
        &mut self,
        player_position: &Vec3,
        load_radius: i32,
        terrain: &WorldTerrain,
    ) {
        let block_x = std::cmp::max(player_position.x.floor() as i32, 0) as u32;
        let block_z = std::cmp::max(player_position.z.floor() as i32, 0) as u32;
        let chunk_x = block_x / 16;
        let chunk_z = block_z / 16;
        let load_radius_u32 = load_radius as u32;

        let start_x = chunk_x.saturating_sub(load_radius_u32);
        let end_x = chunk_x.saturating_add(load_radius_u32);
        let start_z = chunk_z.saturating_sub(load_radius_u32);
        let end_z = chunk_z.saturating_add(load_radius_u32);

        // Simple square load around player
        for cx in start_x..=end_x {
            for cz in start_z..=end_z {
                self.load_cell(cx, cz, terrain);
            }
        }

        // Unload chunks outside radius
        let mut to_remove = Vec::new();
        for key in self.loaded_cells.keys() {
            if key.x < start_x || key.x > end_x || key.y < start_z || key.y > end_z {
                to_remove.push(*key);
            }
        }

        let mut removed_any = false;
        for key in to_remove {
            if self.loaded_cells.remove(&key).is_some() {
                removed_any = true;
            }
        }

        if removed_any {
            self.version = self.version.wrapping_add(1);
        }
    }
}
