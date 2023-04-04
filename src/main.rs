use ruxel::Ruxel;

#[pollster::main]
async fn main() {
    let ruxel = Ruxel::new().await;
    pollster::block_on(ruxel.run());
}
