use anyhow::{Context, Result};
use async_walkdir::WalkDir;
use futures_lite::stream::StreamExt;
use log::*;
use tokio::fs::File;

pub mod entropy;

pub async fn search_dir(directory: &'static str) {
    let mut counter = 0;
    let start = std::time::Instant::now();
    let mut entries = WalkDir::new(directory); //.filter(screen_file);

    loop {
        match entries.next().await {
            Some(Ok(entry)) => {
                counter += 1;
                if counter % 10000 == 0 {
                    let duration = start.elapsed();
                    trace!("Scanned {counter} files in {:?}", duration);
                }
                let entry_path = entry.path().display().to_string();
                let _ = check_file(&entry_path).await;
            }
            Some(Err(_e)) => {
                // ignore files we cannot open
            }
            None => break,
        }
    }

    let duration = start.elapsed();
    info!("Time taken to find entries: {:?}", duration);
    info!("Checked {} files", counter);
}

async fn check_file(filename: &str) -> Result<()> {
    let metadata = tokio::fs::metadata(filename)
        .await
        .context("Could not open file")?;
    let length = metadata.len();
    if metadata.is_dir() || length % 512 != 0 || length < 1024 * 2 {
        return Ok(());
    }
    let mut file = File::open(filename)
        .await
        .context("Could not open file for tokio")?;

    let mut entropy = entropy::Entropy::new_from_file(&mut file).await?;
    let shannon = entropy
        .shannon()
        .await
        .context("could not get shannon entropy")?;
    if shannon < 7.95 {
        return Ok(());
    }

    let file_type = infer::get_from_path(filename);

    match file_type {
        Ok(Some(found_type)) => {
            trace!("(possible) {} {} {:?}", shannon, filename, found_type);
        }
        Ok(None) => {
            println!("{} {}", shannon, filename);
        }
        Err(e) => {
            error!("Error checking file type for {filename} {:?}", e);
        }
    }

    Ok(())
}
