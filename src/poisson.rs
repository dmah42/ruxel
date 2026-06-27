use crate::terrain::{TerrainData, WATER_LEVEL};
use crate::trees;
use glam::Vec2;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone)]
pub(crate) struct AdaptivePoisson {
    seed: u32,
}

impl AdaptivePoisson {
    pub(crate) fn new(seed: u32) -> Self {
        Self { seed }
    }

    fn get_radius<F>(point: [f64; 2], terrain_func: &F) -> f32
    where
        F: Fn([f64; 2]) -> TerrainData,
    {
        let tdata = terrain_func(point);
        if tdata.height <= WATER_LEVEL {
            return f32::INFINITY;
        }

        let temp = tdata.temperature;
        let moist = tdata.moisture;
        let height = tdata.height;

        if height > crate::terrain::TREELINE_ALTITUDE {
            return f32::INFINITY; // Above the treeline, nothing grows
        }

        // Helper to check for nearby water
        let check_water_nearby = || {
            let dist = 15.0;
            let diag = dist * std::f64::consts::FRAC_1_SQRT_2;
            let points_to_check = [
                [point[0] + dist, point[1]],
                [point[0] - dist, point[1]],
                [point[0], point[1] + dist],
                [point[0], point[1] - dist],
                [point[0] + diag, point[1] + diag],
                [point[0] - diag, point[1] + diag],
                [point[0] + diag, point[1] - diag],
                [point[0] - diag, point[1] - diag],
            ];

            points_to_check.iter().any(|&p| {
                let d = terrain_func(p);
                d.height <= WATER_LEVEL
            })
        };

        trees::get_vegetation_radius(temp, moist, height, check_water_nearby())
    }

    // Helper to get the deterministic point for a grid cell
    fn get_cell_point(&self, cell_x: u32, cell_z: u32) -> (Vec2, u64) {
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        cell_x.hash(&mut hasher);
        cell_z.hash(&mut hasher);
        let h = hasher.finish();

        // Use the hash to generate normalized coordinates inside the cell [0, 1]
        let nx = ((h & 0xFFFFFFFF) as f64) / (0xFFFFFFFFu32 as f64);
        let nz = ((h >> 32) as f64) / (0xFFFFFFFFu32 as f64);

        let cell_size = 3.5;
        let world_x = (cell_x as f64 + nx) * cell_size;
        let world_z = (cell_z as f64 + nz) * cell_size;

        (Vec2::new(world_x as f32, world_z as f32), h)
    }

    pub(crate) fn generate_for_chunk<F>(
        &self,
        chunk_x: u32,
        chunk_z: u32,
        terrain_func: &F,
    ) -> Vec<Vec2>
    where
        F: Fn([f64; 2]) -> TerrainData,
    {
        let mut points = Vec::new();
        let cell_size = 3.5;

        let chunk_world_x = (chunk_x * 16) as f64;
        let chunk_world_z = (chunk_z * 16) as f64;

        let start_cell_x = (chunk_world_x / cell_size).floor() as u32;
        let start_cell_z = (chunk_world_z / cell_size).floor() as u32;
        let end_cell_x = ((chunk_world_x + 16.0) / cell_size).ceil() as u32;
        let end_cell_z = ((chunk_world_z + 16.0) / cell_size).ceil() as u32;

        for cx in start_cell_x..=end_cell_x {
            for cz in start_cell_z..=end_cell_z {
                let (p, h) = self.get_cell_point(cx, cz);

                // Only emit if it actually falls strictly inside this chunk
                if (p.x as f64) < chunk_world_x
                    || (p.x as f64) >= chunk_world_x + 16.0
                    || (p.y as f64) < chunk_world_z
                    || (p.y as f64) >= chunk_world_z + 16.0
                {
                    continue;
                }

                let r = Self::get_radius([p.x as f64, p.y as f64], terrain_func);

                if r.is_infinite() {
                    continue;
                }

                // Check neighbors
                let search_radius_cells = (r as f64 / cell_size).ceil() as u32;
                let mut valid = true;

                let min_nx = cx.saturating_sub(search_radius_cells);
                let max_nx = cx.saturating_add(search_radius_cells);
                let min_nz = cz.saturating_sub(search_radius_cells);
                let max_nz = cz.saturating_add(search_radius_cells);

                for nx in min_nx..=max_nx {
                    for nz in min_nz..=max_nz {
                        if nx == cx && nz == cz {
                            continue;
                        }

                        let (np, nh) = self.get_cell_point(nx, nz);

                        let nr = Self::get_radius([np.x as f64, np.y as f64], terrain_func);

                        let required_dist = r.max(nr);
                        let dist = p.distance(np);

                        if dist < required_dist {
                            // If they overlap, the one with the higher hash wins
                            if nh > h {
                                valid = false;
                                break;
                            }
                        }
                    }
                    if !valid {
                        break;
                    }
                }

                if valid {
                    points.push(p);
                }
            }
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{Biome, WorldTerrain};

    #[test]
    fn test_poisson_determinism() {
        let terrain = WorldTerrain::new(12345);
        let poisson = AdaptivePoisson::new(12345);

        let terrain_func = |p: [f64; 2]| terrain.get(p);
        let points_a = poisson.generate_for_chunk(10, 5, &terrain_func);
        let points_b = poisson.generate_for_chunk(10, 5, &terrain_func);

        assert_eq!(points_a.len(), points_b.len());
        for (a, b) in points_a.iter().zip(points_b.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
        }
    }

    fn generate_biome_bitmap<F>(target_biome: Biome, filename: &str, terrain_func: F)
    where
        F: Fn([f64; 2]) -> TerrainData,
    {
        let poisson = AdaptivePoisson::new(12345);
        let width = 4096;
        let height = 4096;
        let mut img = image::ImageBuffer::new(width, height);

        // Fill background with heightmap
        for x in 0..width {
            for z in 0..height {
                let d = terrain_func([x as f64, z as f64]);
                if d.height <= WATER_LEVEL {
                    // Distinct blue for water
                    let depth = (WATER_LEVEL - d.height).clamp(0.0, 30.0);
                    let b = 255 - (depth as u8 * 4);
                    img.put_pixel(x, z, image::Rgb([0, 0, b]));
                } else {
                    // Dimmer gray for land
                    let intensity =
                        ((d.height - WATER_LEVEL) / 80.0 * 255.0).clamp(0.0, 255.0) as u8;
                    if d.moisture > 0.5 {
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
                let points = poisson.generate_for_chunk(cx, cz, &terrain_func);

                for p in points {
                    let px = p.x.round() as u32;
                    let pz = p.y.round() as u32;

                    if px < width && pz < height {
                        // TODO: color by tree type
                        let color: [u8; 3] = match target_biome {
                            Biome::Plains => [255, 140, 0], // Dark Orange (contrast with green grove)
                            Biome::Hills => [150, 150, 0],  // Olive
                            Biome::Mountains => [200, 200, 200], // White/Grey
                            Biome::Desert => [255, 255, 0], // Cactus (Yellow)
                            Biome::Ocean => [0, 0, 255],    // Blue
                        };

                        // Draw a small 3x3 square for visibility
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

        let out_path = filename;
        img.save(&out_path).unwrap();
    }

    #[test]
    fn test_poisson_bitmap_plains() {
        let gen = crate::terrain::PlainsTerrain::new(42);
        let scale = crate::terrain::WorldTerrain::WORLD_SCALE;
        generate_biome_bitmap(
            Biome::Plains,
            "test_outputs/poisson_bitmap_plains.bmp",
            |p| {
                let h = gen.get([p[0] / scale, p[1] / scale]);
                TerrainData {
                    height: h,
                    biome: Biome::Plains,
                    moisture: 0.5,
                    temperature: 0.5,
                }
            },
        );
    }

    #[test]
    fn test_poisson_bitmap_mountains() {
        let gen = crate::terrain::MountainTerrain::new(42);
        let scale = crate::terrain::WorldTerrain::WORLD_SCALE;
        generate_biome_bitmap(
            Biome::Mountains,
            "test_outputs/poisson_bitmap_mountains.bmp",
            |p| TerrainData {
                height: gen.get([p[0] / scale, p[1] / scale]),
                biome: Biome::Mountains,
                moisture: 0.5,
                temperature: 0.5,
            },
        );
    }

    #[test]
    fn test_poisson_bitmap_hills() {
        let gen = crate::terrain::HillsTerrain::new(42);
        let scale = crate::terrain::WorldTerrain::WORLD_SCALE;
        generate_biome_bitmap(Biome::Hills, "test_outputs/poisson_bitmap_hills.bmp", |p| {
            let h = gen.get([p[0] / scale, p[1] / scale]);
            TerrainData {
                height: h,
                biome: Biome::Hills,
                moisture: 0.5,
                temperature: 0.5,
            }
        });
    }

    #[test]
    fn test_poisson_bitmap_desert() {
        let gen = crate::terrain::DesertTerrain::new(42);
        let scale = crate::terrain::WorldTerrain::WORLD_SCALE;
        generate_biome_bitmap(
            Biome::Desert,
            "test_outputs/poisson_bitmap_desert.bmp",
            |p| TerrainData {
                height: gen.get([p[0] / scale, p[1] / scale]),
                biome: Biome::Desert,
                moisture: 0.0,
                temperature: 1.0,
            },
        );
    }
}
