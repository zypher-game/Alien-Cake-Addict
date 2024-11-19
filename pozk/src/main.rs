use z4_pozk::Engine;

mod handler;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    Engine::<handler::GameHandler>::run()
        .await
        .expect("Down");
}
