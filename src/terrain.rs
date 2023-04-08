use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

pub fn gen(seed: u32) -> impl NoiseFn<f64, 2> {
    Fbm::<Perlin>::new(seed)
        .set_frequency(1.0)
        .set_persistence(0.5)
        .set_lacunarity(2.208984375)
        .set_octaves(14)
}
