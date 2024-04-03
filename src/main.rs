use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    let args = Box::new(args);
    let args: &mut Vec<String> = Box::leak(args);

    let _ = tchunt_rs::search_dir(&args[1]).await;

    Ok(())
}
