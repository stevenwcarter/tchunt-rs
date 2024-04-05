use anyhow::{Context, Result};
use async_walkdir::WalkDir;
use futures::stream::StreamExt;
use infer::Type;
use log::*;
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::Mutex;

pub mod entropy;

pub async fn search_dir(directory: &'static str) -> Result<()> {
    let finished = Arc::new(Mutex::new(false));
    let thread_finished = finished.clone();

    let search_thread = tokio::spawn(async move {
        let mut counter = 0;
        let start = std::time::Instant::now();
        let mut entries = WalkDir::new(directory); //.filter(screen_file);

        loop {
            match entries.next().await {
                Some(Ok(entry)) => {
                    counter += 1;
                    if counter % 1000 == 0 {
                        let duration = start.elapsed();
                        trace!("Scanned {counter} files in {:?}", duration);
                    }
                    // if screen_file(&entry).await {
                    let entry_path = entry.path().display().to_string();
                    let _ = check_file(&entry_path).await;
                    // }
                }
                Some(Err(_e)) => {
                    // eprintln!("error: {}", e);
                }
                None => break,
            }
        }

        let duration = start.elapsed();
        info!("Time taken to find entries: {:?}", duration);
        info!("Checked {} files", counter);
        *thread_finished.lock().await = true;
    });

    let _ = search_thread.await;

    Ok(())
}

pub async fn check_file(filename: &str) -> Result<()> {
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

    let shannon = entropy::shannon(&mut file)
        .await
        .context("could not get shannon entropy")?;
    if shannon < 7.9 {
        return Ok(());
    }
    let file_type = get_file_type(filename).await;

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

pub async fn get_file_type(filename: &str) -> Result<Option<Type>> {
    infer::get_from_path(filename).context("could not read file for type inference")
}
