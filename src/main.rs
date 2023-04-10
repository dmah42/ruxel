use rand::Rng;
use ruxel::Ruxel;

#[pollster::main]
async fn main() {
    let mut rng = rand::thread_rng();
    let seed = rng.gen::<u32>();
    println!("using seed {seed:X}");

    let ruxel = Ruxel::new(seed).await;
    pollster::block_on(ruxel.run());
}
