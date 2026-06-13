use rand::Rng;
use ruxel::Ruxel;

#[pollster::main]
async fn main() {
    let mut rng = rand::thread_rng();
    let seed = rng.gen::<u32>();
    println!("using seed {seed:X}");

    match Ruxel::new(seed).await {
        Ok(ruxel) => pollster::block_on(ruxel.run()),
        Err(e) => eprintln!("Fatal error: failed to initialize application: {}", e),
    }
}
