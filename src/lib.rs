use anyhow::{Context, Result};
use async_walkdir::WalkDir;
use futures_lite::stream::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::*;

pub mod entropy;

pub async fn search_dir(directory: &str) {
    const MAX_CONCURRENT: usize = 64;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut tasks: JoinSet<()> = JoinSet::new();
    let mut counter = 0u64;
    let start = std::time::Instant::now();
    let mut entries = WalkDir::new(directory);

    loop {
        match entries.next().await {
            Some(Ok(entry)) => {
                counter += 1;
                if counter.is_multiple_of(10000) {
                    trace!("Scanned {counter} files in {:?}", start.elapsed());
                }
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("semaphore is never closed");
                let path = entry.path().to_path_buf();
                tasks.spawn(async move {
                    let _permit = permit;
                    if let Err(e) = check_file(&path).await {
                        debug!("Error checking {:?}: {:?}", path, e);
                    }
                });
            }
            Some(Err(e)) => {
                debug!("WalkDir error: {:?}", e);
            }
            None => break,
        }
    }

    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            warn!("Task panicked: {:?}", e);
        }
    }

    let duration = start.elapsed();
    info!("Time taken to find entries: {:?}", duration);
    info!("Checked {} files", counter);
}

async fn check_file(path: &Path) -> Result<()> {
    let metadata = tokio::fs::metadata(path)
        .await
        .context("Could not stat file")?;
    let length = metadata.len();
    // files must be at least 4 MB and a multiple of 512 bytes
    if metadata.is_dir() || length % 512 != 0 || length < 1024 * 1024 * 4 {
        return Ok(());
    }

    let file = File::open(path).await.context("Could not open file")?;
    let file_len = usize::try_from(length).context("file length overflows usize")?;
    let mut entropy = entropy::Entropy::new(file, file_len);
    let shannon = entropy
        .shannon()
        .await
        .context("could not get shannon entropy")?;
    if shannon < 7.985 {
        return Ok(());
    }

    match infer::get(entropy.head_bytes()) {
        Some(found_type) => {
            trace!("(possible) {} {} {:?}", shannon, path.display(), found_type);
        }
        None => {
            // intentional: candidates go to stdout for piping; tracing diagnostics go to stderr
            println!("{} {}", shannon, path.display());
        }
    }

    Ok(())
}
