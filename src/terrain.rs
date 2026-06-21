use noise::{Fbm, MultiFractal, NoiseFn, Perlin, Simplex};
use std::fmt;

fn fbm_bound(octaves: usize, persistence: f64) -> f64 {
    (0..octaves).map(|i| persistence.powi(i as i32)).sum()
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Ocean,
    Plains,
    Hills,
    Mountains,
    Desert,
}

impl fmt::Display for Biome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Biome {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ocean" => Some(Biome::Ocean),
            "plains" => Some(Biome::Plains),
            "hills" => Some(Biome::Hills),
            "mountains" => Some(Biome::Mountains),
            "desert" => Some(Biome::Desert),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
struct BiomeWeights {
    plains: f64,
    hills: f64,
    desert: f64,
    mountains: f64,
}

#[derive(Clone)]
pub struct WorldTerrain {
    continent_noise: Fbm<Perlin>,
    temperature_noise: Fbm<Perlin>,
    moisture_noise: Fbm<Perlin>,
    ocean: OceanTerrain,
    plains: PlainsTerrain,
    hills: HillsTerrain,
    mountains: MountainTerrain,
    desert: DesertTerrain,
}

impl WorldTerrain {
    pub fn new(seed: u32) -> Self {
        Self {
            continent_noise: Fbm::<Perlin>::new(seed.wrapping_add(100))
                .set_frequency(0.1)
                .set_octaves(3)
                .set_persistence(0.45),
            temperature_noise: Fbm::<Perlin>::new(seed.wrapping_add(101))
                .set_frequency(0.15)
                .set_octaves(4)
                .set_persistence(0.5),
            moisture_noise: Fbm::<Perlin>::new(seed.wrapping_add(102))
                .set_frequency(0.15)
                .set_octaves(4)
                .set_persistence(0.5),
            ocean: OceanTerrain::new(seed.wrapping_add(150)),
            plains: PlainsTerrain::new(seed.wrapping_add(200)),
            hills: HillsTerrain::new(seed.wrapping_add(300)),
            mountains: MountainTerrain::new(seed.wrapping_add(400)),
            desert: DesertTerrain::new(seed.wrapping_add(500)),
        }
    }

    fn get_land_blend(&self, point: [f64; 2]) -> (Biome, BiomeWeights) {
        let temp_raw = self.temperature_noise.get(point);

        let moist_raw = self.moisture_noise.get(point);

        let temp_norm = ((temp_raw + 1.0) / 2.0).clamp(0.0, 1.0);
        let moist_norm = ((moist_raw + 1.0) / 2.0).clamp(0.0, 1.0);

        // Calculate distance to each biome center in the parameter space
        // Mountains: Cold (0.0), Any Moisture
        let d_mountains = temp_norm;
        // Desert: Hot (1.0), Dry (0.0)
        let d_desert = ((temp_norm - 1.0).powi(2) + (moist_norm - 0.0).powi(2)).sqrt();
        // Plains: Moderate (0.5), Moderate (0.5)
        let d_plains = ((temp_norm - 0.5).powi(2) + (moist_norm - 0.5).powi(2)).sqrt();
        // Hills: Hot (1.0), Wet (1.0)
        let d_hills = ((temp_norm - 1.0).powi(2) + (moist_norm - 1.0).powi(2)).sqrt();

        // Find the closest biome
        let min_d = d_mountains.min(d_desert).min(d_plains).min(d_hills);

        let mut primary_biome = Biome::Mountains;
        if min_d == d_hills {
            primary_biome = Biome::Hills;
        }
        if min_d == d_desert {
            primary_biome = Biome::Desert;
        }
        if min_d == d_plains {
            primary_biome = Biome::Plains;
        }
        if min_d == d_mountains {
            primary_biome = Biome::Mountains;
        }

        // Blend weights based on distance difference from the minimum distance
        let blend_radius = 0.1; // This defines the width of the transition zone
        let diff_mountains = d_mountains - min_d;
        let diff_desert = d_desert - min_d;
        let diff_plains = d_plains - min_d;
        let diff_hills = d_hills - min_d;

        // smoothstep ensures that the weight drops smoothly to 0 at the blend_radius
        let w_mountains = smoothstep(0.0, blend_radius, blend_radius - diff_mountains);
        let w_desert = smoothstep(0.0, blend_radius, blend_radius - diff_desert);
        let w_plains = smoothstep(0.0, blend_radius, blend_radius - diff_plains);
        let w_hills = smoothstep(0.0, blend_radius, blend_radius - diff_hills);

        let sum = w_mountains + w_desert + w_plains + w_hills;

        let weights = BiomeWeights {
            mountains: w_mountains / sum,
            desert: w_desert / sum,
            plains: w_plains / sum,
            hills: w_hills / sum,
        };

        (primary_biome, weights)
    }

    fn get_shore_t(&self, point: [f64; 2]) -> f64 {
        let val = self.continent_noise.get(point);
        // val is roughly [-1.0, 1.0]. Map it to [0.0, 1.0]
        let t = ((val + 1.0) / 2.0).clamp(0.0, 1.0);
        // Adjust threshold so that ~30-40% is ocean.
        // Let's say if t < 0.45, it is ocean (0.0).
        // The transition to full land (1.0) will happen quickly over a small margin.
        if t < 0.40 {
            0.0
        } else if t < 0.50 {
            (t - 0.40) / 0.10
        } else {
            1.0
        }
    }

    pub fn get(&self, point: [f64; 2]) -> (f64, Biome) {
        let shore_t = self.get_shore_t(point);

        let (mut primary_biome, weights) = self.get_land_blend(point);
        if shore_t == 0.0 {
            primary_biome = Biome::Ocean;
        }

        let mut land_height = 0.0;
        if weights.plains > 0.0 {
            land_height += weights.plains * self.plains.get(point);
        }
        if weights.hills > 0.0 {
            land_height += weights.hills * self.hills.get(point);
        }
        if weights.desert > 0.0 {
            land_height += weights.desert * self.desert.get(point);
        }
        if weights.mountains > 0.0 {
            land_height += weights.mountains * self.mountains.get(point);
        }

        let ocean_abyss = self.ocean.get(point);
        let final_height = if shore_t >= 1.0 {
            land_height
        } else {
            let shore_t_norm = shore_t;
            if shore_t_norm > 0.8 {
                let local_t = (shore_t_norm - 0.8) / 0.2;
                let w = local_t * local_t * (3.0 - 2.0 * local_t);
                31.0 * (1.0 - w) + land_height * w
            } else if shore_t_norm > 0.3 {
                let local_t = (shore_t_norm - 0.3) / 0.5;
                let w = local_t * local_t * (3.0 - 2.0 * local_t);
                24.0 * (1.0 - w) + 31.0 * w
            } else {
                let local_t = shore_t_norm / 0.3;
                let w = local_t * local_t * (3.0 - 2.0 * local_t);
                ocean_abyss * (1.0 - w) + 24.0 * w
            }
        };

        (final_height, primary_biome)
    }

    pub fn is_pure_biome(&self, point: [f64; 2], target: Biome) -> bool {
        let shore_t = self.get_shore_t(point);
        if target == Biome::Ocean {
            return shore_t == 0.0;
        }
        if shore_t < 1.0 {
            return false; // Mixed with ocean
        }

        let (primary, weights) = self.get_land_blend(point);
        if primary != target {
            return false;
        }

        let w = match target {
            Biome::Plains => weights.plains,
            Biome::Hills => weights.hills,
            Biome::Desert => weights.desert,
            Biome::Mountains => weights.mountains,
            Biome::Ocean => 0.0,
        };

        w > 0.99
    }

    pub fn find_closest_pure_biome(
        &self,
        start_x: f64,
        start_z: f64,
        target: Biome,
    ) -> Option<[f64; 2]> {
        let step_size = 32.0 / 384.0;
        let max_steps = 400 * 400;
        let start_point = [start_x / 384.0, start_z / 384.0];

        let mut x = 0;
        let mut z = 0;
        let mut dx = 0;
        let mut dz = -1;

        for _ in 0..max_steps {
            let px = start_point[0] + (x as f64 * step_size);
            let pz = start_point[1] + (z as f64 * step_size);

            let point = [px, pz];
            if self.is_pure_biome(point, target) {
                return Some([px * 384.0, pz * 384.0]);
            }

            if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
                let temp = dx;
                dx = -dz;
                dz = temp;
            }

            x += dx;
            z += dz;
        }
        None
    }

    pub fn biome_blend_string(&self, point: [f64; 2]) -> String {
        let shore_t = self.get_shore_t(point);

        if shore_t == 0.0 {
            return "100% Ocean".to_string();
        }

        let (_, weights) = self.get_land_blend(point);

        let land_pct = (shore_t * 100.0).round() as i32;
        let ocean_pct = 100 - land_pct;

        let mut b_strs = Vec::new();
        if ocean_pct > 0 {
            b_strs.push(format!("{}% Ocean", ocean_pct));
        }

        let mut add_land = |name: &str, w: f64| {
            if w > 0.0 {
                let p = ((w * 100.0) * shore_t).round() as i32;
                if p > 0 {
                    b_strs.push(format!("{}% {}", p, name));
                }
            }
        };

        add_land("Plains", weights.plains);
        add_land("Hills", weights.hills);
        add_land("Desert", weights.desert);
        add_land("Mountains", weights.mountains);

        b_strs.join(", ")
    }
}

// -----------------------------------------------------------------------------
// Biome Generators
// -----------------------------------------------------------------------------

#[derive(Clone)]
struct OceanTerrain {
    noise: Fbm<Simplex>,
    bound: f64,
}

impl OceanTerrain {
    fn new(seed: u32) -> Self {
        let octaves = 4;
        let persistence = 0.5;
        Self {
            noise: Fbm::<Simplex>::new(seed)
                .set_frequency(1.0)
                .set_persistence(persistence)
                .set_octaves(octaves),
            bound: fbm_bound(octaves, persistence),
        }
    }

    fn get(&self, point: [f64; 2]) -> f64 {
        let val = self.noise.get(point);
        let norm = (val + self.bound) / (2.0 * self.bound);
        norm * 16.0
    }
}

#[derive(Clone)]
struct PlainsTerrain {
    base_noise: Fbm<Perlin>,
    base_bound: f64,
    detail_noise: Fbm<Perlin>,
    detail_bound: f64,
}

impl PlainsTerrain {
    fn new(seed: u32) -> Self {
        let base_octaves = 4;
        let base_persistence = 0.5;
        let detail_octaves = 3;
        let detail_persistence = 0.4;

        Self {
            base_noise: Fbm::<Perlin>::new(seed)
                .set_frequency(0.5)
                .set_persistence(base_persistence)
                .set_octaves(base_octaves),
            base_bound: fbm_bound(base_octaves, base_persistence),
            detail_noise: Fbm::<Perlin>::new(seed.wrapping_add(1))
                .set_frequency(2.0)
                .set_persistence(detail_persistence)
                .set_octaves(detail_octaves),
            detail_bound: fbm_bound(detail_octaves, detail_persistence),
        }
    }

    fn get(&self, point: [f64; 2]) -> f64 {
        let base_val = self.base_noise.get(point);
        let detail_val = self.detail_noise.get(point);

        let base_norm = (base_val + self.base_bound) / (2.0 * self.base_bound);
        let detail_norm = (detail_val + self.detail_bound) / (2.0 * self.detail_bound);

        let combined = base_norm * 0.8 + detail_norm * 0.2;
        20.0 + combined * 28.0
    }
}

#[derive(Clone)]
struct HillsTerrain {
    base_noise: Fbm<Perlin>,
    base_bound: f64,
    mask_noise: Fbm<Perlin>,
    mask_bound: f64,
}

impl HillsTerrain {
    fn new(seed: u32) -> Self {
        let base_octaves = 6;
        let base_persistence = 0.5;
        let mask_octaves = 4;
        let mask_persistence = 0.5;

        Self {
            base_noise: Fbm::<Perlin>::new(seed)
                .set_frequency(0.4)
                .set_persistence(0.4)
                .set_lacunarity(2.208984375)
                .set_octaves(base_octaves),
            base_bound: fbm_bound(base_octaves, base_persistence),
            mask_noise: Fbm::<Perlin>::new(seed.wrapping_add(1))
                .set_frequency(0.15)
                .set_persistence(mask_persistence)
                .set_octaves(mask_octaves),
            mask_bound: fbm_bound(mask_octaves, mask_persistence),
        }
    }

    fn get(&self, point: [f64; 2]) -> f64 {
        let base_raw = self.base_noise.get(point);
        let mask_raw = self.mask_noise.get(point);

        let base_val = (base_raw + self.base_bound) / (2.0 * self.base_bound);
        let mask_val = (mask_raw + self.mask_bound) / (2.0 * self.mask_bound);

        let exponent = 1.0 + (mask_val * 3.5);
        let final_val = base_val.powf(exponent);

        24.0 + final_val * 100.0
    }
}

#[derive(Clone)]
struct MountainTerrain {
    base_noise: Fbm<Perlin>,
    base_bound: f64,
    mask_noise: Fbm<Perlin>,
    mask_bound: f64,
}

impl MountainTerrain {
    fn new(seed: u32) -> Self {
        let base_octaves = 14;
        let base_persistence = 0.5;
        let mask_octaves = 4;
        let mask_persistence = 0.5;

        Self {
            base_noise: Fbm::<Perlin>::new(seed)
                .set_frequency(1.0)
                .set_persistence(base_persistence)
                .set_lacunarity(2.208984375)
                .set_octaves(base_octaves),
            base_bound: fbm_bound(base_octaves, base_persistence),
            mask_noise: Fbm::<Perlin>::new(seed.wrapping_add(1))
                .set_frequency(0.15)
                .set_persistence(mask_persistence)
                .set_octaves(mask_octaves),
            mask_bound: fbm_bound(mask_octaves, mask_persistence),
        }
    }

    fn get(&self, point: [f64; 2]) -> f64 {
        let base_raw = self.base_noise.get(point);
        let mask_raw = self.mask_noise.get(point);

        let base_val = (base_raw + self.base_bound) / (2.0 * self.base_bound);
        let mask_val = (mask_raw + self.mask_bound) / (2.0 * self.mask_bound);

        let exponent = 1.0 + (mask_val * 4.5);
        let final_val = base_val.powf(exponent);

        24.0 + final_val * 200.0
    }
}

fn hash2(x: u32, y: u32, seed: u32) -> [f64; 2] {
    let mut h = seed;
    h ^= x.wrapping_mul(0x9E3779B9);
    h ^= y.wrapping_mul(0x85EBCA6B);
    h ^= h >> 16;
    h = h.wrapping_mul(0x732A12AB);
    h ^= h >> 15;

    let fx = (h & 0xFFFF) as f64 / 65535.0;
    let fy = ((h >> 16) & 0xFFFF) as f64 / 65535.0;
    [fx, fy]
}

#[derive(Clone)]
struct DesertTerrain {
    base_noise: Fbm<Perlin>,
    base_bound: f64,
    seed: u32,
}

impl DesertTerrain {
    fn new(seed: u32) -> Self {
        Self {
            base_noise: Fbm::<Perlin>::new(seed.wrapping_add(2))
                .set_frequency(0.5)
                .set_octaves(3),
            base_bound: fbm_bound(3, 0.5),
            seed,
        }
    }

    fn get(&self, point: [f64; 2]) -> f64 {
        // Generate base desert floor
        let base_raw = self.base_noise.get(point);
        let base_norm = ((base_raw + self.base_bound) / (2.0 * self.base_bound)).clamp(0.0, 1.0);

        // Voronoi Barchan Dune Generation (Splatting approach)
        // Instead of picking the *single* closest seed (which causes sharp cell boundaries),
        // we evaluate the dune height from ALL 9 neighboring cells and take the maximum.
        // This makes the dunes perfectly smooth with no grid lines or cell edges!
        let freq = 0.8;
        let px = point[0] * freq;
        let pz = point[1] * freq;

        let ix = px.floor();
        let iz = pz.floor();
        let fx = px - ix;
        let fz = pz - iz;

        let mut max_dune_height = 0.0_f64;

        for dz in -1..=1 {
            for dx in -1..=1 {
                let cx = ix + dx as f64;
                let cz = iz + dz as f64;

                let ux = cx as i64 as u32;
                let uz = cz as i64 as u32;

                let p = hash2(ux, uz, self.seed);

                // Vector from pixel to this neighbor's seed point
                let vx = (dx as f64) + p[0] - fx;
                let vz = (dz as f64) + p[1] - fz;

                let local_dx = vx;
                let local_dz = vz;

                let cross_dist = local_dz.abs();

                // Horns curve downwind.
                let horn_sweep = 0.8;
                let local_x = local_dx + cross_dist * cross_dist * horn_sweep;

                let mut profile = 0.0;

                if local_x >= 0.0 {
                    // Windward slope (gentle rise)
                    if local_x < 0.8 {
                        let t = 1.0 - (local_x / 0.8);
                        profile = t * t * (3.0 - 2.0 * t);
                    }
                } else {
                    // Slip face (sharp drop)
                    if local_x > -0.2 {
                        let t = 1.0 - (local_x.abs() / 0.2);
                        profile = t * t * (3.0 - 2.0 * t);
                    }
                }

                // Taper height sideways so it blends into the desert floor at the edges
                let taper = (1.0 - cross_dist * 1.5).clamp(0.0, 1.0);
                let height = profile * taper * taper;

                if height > max_dune_height {
                    max_dune_height = height;
                }
            }
        }

        // Sand ripple
        let ripple = (base_raw * 0.1).abs();

        26.0 + base_norm * 8.0 + max_dune_height * 22.0 + ripple
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terraced_noise_blend() {
        let terrain = WorldTerrain::new(12345);

        for i in 0..100 {
            let point = [i as f64 / 10.0, i as f64 / 10.0];
            let (primary, weights) = terrain.get_land_blend(point);

            let sum = weights.plains + weights.hills + weights.desert + weights.mountains;
            assert!(
                (sum - 1.0).abs() < 0.001,
                "Weights should sum to 1.0, got {}",
                sum
            );

            let mut w_array = [
                (Biome::Plains, weights.plains),
                (Biome::Hills, weights.hills),
                (Biome::Desert, weights.desert),
                (Biome::Mountains, weights.mountains),
            ];
            w_array.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let max_calculated_weight = match primary {
                Biome::Plains => weights.plains,
                Biome::Hills => weights.hills,
                Biome::Desert => weights.desert,
                Biome::Mountains => weights.mountains,
                Biome::Ocean => 0.0,
            };
            assert!(
                (max_calculated_weight - w_array[0].1).abs() < 0.001,
                "Primary biome must have the highest weight, even on ties"
            );
        }
    }

    #[test]
    fn test_biome_coverage() {
        let terrain = WorldTerrain::new(12345);
        let mut pure_blocks = 0;
        let mut total_blocks = 0;

        let mut biome_counts = std::collections::HashMap::new();

        let grid_size = 10000;
        let mut img = image::RgbImage::new(grid_size as u32, grid_size as u32);

        for x in 0..grid_size {
            for z in 0..grid_size {
                let point = [x as f64 / 384.0, z as f64 / 384.0];
                total_blocks += 1;

                let shore_t = terrain.get_shore_t(point);
                let (primary_land, land_weights) = terrain.get_land_blend(point);

                let final_biome = if shore_t < 0.5 {
                    Biome::Ocean
                } else {
                    primary_land
                };

                *biome_counts.entry(final_biome).or_insert(0) += 1;

                let primary_w = match final_biome {
                    Biome::Plains => shore_t * land_weights.plains,
                    Biome::Hills => shore_t * land_weights.hills,
                    Biome::Desert => shore_t * land_weights.desert,
                    Biome::Mountains => shore_t * land_weights.mountains,
                    Biome::Ocean => 1.0 - shore_t,
                };

                if primary_w > 0.9 {
                    pure_blocks += 1;
                }

                let ocean_w = 1.0 - shore_t;
                let plains_w = shore_t * land_weights.plains;
                let hills_w = shore_t * land_weights.hills;
                let desert_w = shore_t * land_weights.desert;
                let mountains_w = shore_t * land_weights.mountains;

                let r = (ocean_w * 0.0
                    + plains_w * 144.0
                    + hills_w * 34.0
                    + desert_w * 237.0
                    + mountains_w * 200.0) as u8;
                let g = (ocean_w * 105.0
                    + plains_w * 238.0
                    + hills_w * 139.0
                    + desert_w * 201.0
                    + mountains_w * 200.0) as u8;
                let b = (ocean_w * 148.0
                    + plains_w * 144.0
                    + hills_w * 34.0
                    + desert_w * 175.0
                    + mountains_w * 200.0) as u8;

                img.put_pixel(x as u32, z as u32, image::Rgb([r, g, b]));
            }
        }

        img.save("biome_map.bmp")
            .expect("Failed to save biome_map.bmp");

        assert!(total_blocks > 0, "No blocks generated in the test area.");
        let pure_pct = pure_blocks as f64 / total_blocks as f64;
        println!("Pure biome coverage: {:.2}%", pure_pct * 100.0);
        let biome_pcts: Vec<String> = biome_counts
            .iter()
            .map(|(b, c)| format!("{:?}: {:.2}%", b, *c as f64 / total_blocks as f64 * 100.0))
            .collect();
        println!("Biome distribution: {:?}", biome_pcts);

        // Ensure at least 60% of the land is pure (not in a transition zone)
        assert!(
            pure_pct > 0.60,
            "Not enough pure biome coverage! Only {:.2}% is pure.",
            pure_pct * 100.0
        );
    }

    #[test]
    fn test_continent_map() {
        let terrain = WorldTerrain::new(12345);
        let grid_size = 10000;
        let mut img = image::ImageBuffer::new(grid_size as u32, grid_size as u32);

        let mut ocean_blocks = 0;
        let mut total_blocks = 0;

        for x in 0..grid_size {
            for z in 0..grid_size {
                let point = [x as f64 / 384.0, z as f64 / 384.0];
                total_blocks += 1;

                let shore_t = terrain.get_shore_t(point);
                if shore_t == 0.0 {
                    ocean_blocks += 1;
                }

                let r = (shore_t * 34.0 + (1.0 - shore_t) * 0.0) as u8;
                let g = (shore_t * 139.0 + (1.0 - shore_t) * 105.0) as u8;
                let b = (shore_t * 34.0 + (1.0 - shore_t) * 148.0) as u8;

                img.put_pixel(x as u32, z as u32, image::Rgb([r, g, b]));
            }
        }

        img.save("continent_map.bmp")
            .expect("Failed to save continent_map.bmp");

        let ocean_pct = ocean_blocks as f64 / total_blocks as f64;
        println!("Ocean coverage: {:.2}%", ocean_pct * 100.0);
        // We want roughly 30-40% ocean
        assert!(
            ocean_pct > 0.20 && ocean_pct < 0.50,
            "Ocean coverage {:.2}% is out of bounds (20-50%).",
            ocean_pct * 100.0
        );
    }

    #[test]
    fn test_biome_heightmap_plains() {
        let plains = PlainsTerrain::new(123);
        let grid_size = 500;
        let mut img = image::ImageBuffer::new(grid_size as u32, grid_size as u32);
        for x in 0..grid_size {
            for z in 0..grid_size {
                let h = plains.get([x as f64 / 100.0, z as f64 / 100.0]);
                // Map height roughly 0..100 to 0..255 grayscale
                let c = (h.clamp(0.0, 100.0) * 2.55) as u8;
                img.put_pixel(x as u32, z as u32, image::Rgb([c, c, c]));
            }
        }
        img.save("biome_height_plains.bmp")
            .expect("Failed to save plains heightmap");
    }

    #[test]
    fn test_biome_heightmap_mountains() {
        let mountains = MountainTerrain::new(123);
        let grid_size = 500;
        let mut img = image::ImageBuffer::new(grid_size as u32, grid_size as u32);
        for x in 0..grid_size {
            for z in 0..grid_size {
                let h = mountains.get([x as f64 / 100.0, z as f64 / 100.0]);
                let c = (h.clamp(0.0, 150.0) * 1.7) as u8;
                img.put_pixel(x as u32, z as u32, image::Rgb([c, c, c]));
            }
        }
        img.save("biome_height_mountains.bmp")
            .expect("Failed to save mountain heightmap");
    }

    #[test]
    fn test_biome_heightmap_desert() {
        let desert = DesertTerrain::new(123);
        let grid_size = 500;
        let mut img = image::ImageBuffer::new(grid_size as u32, grid_size as u32);
        for x in 0..grid_size {
            for z in 0..grid_size {
                let h = desert.get([x as f64 / 100.0, z as f64 / 100.0]);
                let c = (h.clamp(0.0, 100.0) * 2.55) as u8;
                img.put_pixel(x as u32, z as u32, image::Rgb([c, c, c]));
            }
        }
        img.save("biome_height_desert.bmp")
            .expect("Failed to save desert heightmap");
    }

    #[test]
    fn test_biome_heightmap_hills() {
        let hills = HillsTerrain::new(123);
        let grid_size = 500;
        let mut img = image::ImageBuffer::new(grid_size as u32, grid_size as u32);
        for x in 0..grid_size {
            for z in 0..grid_size {
                let h = hills.get([x as f64 / 100.0, z as f64 / 100.0]);
                let c = (h.clamp(0.0, 120.0) * 2.12) as u8;
                img.put_pixel(x as u32, z as u32, image::Rgb([c, c, c]));
            }
        }
        img.save("biome_height_hills.bmp")
            .expect("Failed to save hills heightmap");
    }
}
