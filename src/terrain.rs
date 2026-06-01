use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

// Added Clone here to fix the chunk-cloning error
#[derive(Clone)]
pub struct MountainTerrain {
    base_noise: Fbm<Perlin>,
    mask_noise: Fbm<Perlin>,
}

impl MountainTerrain {
    pub fn new(seed: u32) -> Self {
        Self {
            base_noise: Fbm::<Perlin>::new(seed)
                .set_frequency(1.0)
                .set_persistence(0.5)
                .set_lacunarity(2.208984375)
                .set_octaves(14),
            mask_noise: Fbm::<Perlin>::new(seed.wrapping_add(1))
                .set_frequency(0.15)
                .set_octaves(4),
        }
    }
}

impl NoiseFn<f64, 2> for MountainTerrain {
    fn get(&self, point: [f64; 2]) -> f64 {
        let base_raw = self.base_noise.get(point);
        let mask_raw = self.mask_noise.get(point);

        // Map smooth Perlin [-1.0, 1.0] to positive [0.0, 1.0] space
        let base_val = (base_raw + 1.0) / 2.0;
        let mask_val = (mask_raw + 1.0) / 2.0;

        // Apply exponential curves to mountain regions
        let exponent = 1.0 + (mask_val * 3.5);
        let final_val = base_val.powf(exponent);

        // Remap back to standard [-1.0, 1.0] range
        (final_val * 2.0) - 1.0
    }
}
