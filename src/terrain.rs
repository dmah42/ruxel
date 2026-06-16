use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

fn fbm_bound(octaves: usize, persistence: f64) -> f64 {
    (0..octaves).map(|i| persistence.powi(i as i32)).sum()
}

// Added Clone here to fix the chunk-cloning error
#[derive(Clone)]
pub struct MountainTerrain {
    base_noise: Fbm<Perlin>,
    base_bound: f64,
    mask_noise: Fbm<Perlin>,
    mask_bound: f64,
}

impl MountainTerrain {
    pub fn new(seed: u32) -> Self {
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
}

impl NoiseFn<f64, 2> for MountainTerrain {
    fn get(&self, point: [f64; 2]) -> f64 {
        let base_raw = self.base_noise.get(point);
        let mask_raw = self.mask_noise.get(point);

        // Map noise [-bound, bound] to positive [0.0, 1.0] space
        let base_val = (base_raw + self.base_bound) / (2.0 * self.base_bound);
        let mask_val = (mask_raw + self.mask_bound) / (2.0 * self.mask_bound);

        // Apply exponential curves to mountain regions
        let exponent = 1.0 + (mask_val * 3.5);
        let final_val = base_val.powf(exponent);

        // Remap back to standard [-1.0, 1.0] range
        (final_val * 2.0) - 1.0
    }
}
