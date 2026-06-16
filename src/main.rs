use ruxel_lib::Ruxel;

#[pollster::main]
async fn main() {
    match Ruxel::new().await {
        Ok(ruxel) => pollster::block_on(ruxel.run()),
        Err(e) => eprintln!("Fatal error: failed to initialize application: {}", e),
    }
}
