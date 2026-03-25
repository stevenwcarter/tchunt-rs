use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <directory>", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];
    let meta = tokio::fs::metadata(path).await;
    match meta {
        Ok(m) if m.is_dir() => {}
        Ok(_) => {
            eprintln!("Error: '{}' is not a directory", path);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: cannot access '{}': {}", path, e);
            std::process::exit(1);
        }
    }
    tchunt_rs::search_dir(path).await;
    Ok(())
}
