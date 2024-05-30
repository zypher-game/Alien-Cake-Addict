use z4_engine::{Config, Engine};

mod handler;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::from_env().unwrap();
    Engine::<handler::GameHandler>::init(config)
        .run()
        .await
        .expect("Down");
}
