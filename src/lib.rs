use anyhow::{Context, Result};
use async_walkdir::WalkDir;
use futures::stream::StreamExt;
use infer::Type;
use log::*;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::sync::Mutex;

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
    if metadata.is_dir() || length % 512 != 0 || length < 1024 * 1024 * 10 {
        return Ok(());
    }
    let mut file = File::open(filename)
        .await
        .context("Could not open file for tokio")?;

    let shannon = shannon_entropy(&mut file)
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

pub async fn shannon_entropy(file: &mut tokio::fs::File) -> Result<f32> {
    let mut entropy = 0.0;
    let mut counts = [0usize; 256];

    let mut bytes_to_read: usize = 171072;
    let file_len: usize = file.metadata().await?.len() as usize;
    if file_len < bytes_to_read {
        bytes_to_read = file_len;
    }
    let mut bytes_read = bytes_to_read;

    let mut buffer = vec![0; bytes_to_read];
    let mut reader = BufReader::new(file.try_clone().await?);
    reader.read_exact(&mut buffer).await?;

    for byte in buffer {
        counts[byte as usize] += 1;
    }
    if file_len > 2 * bytes_to_read {
        let mut buffer = vec![0; bytes_to_read];
        file.seek(tokio::io::SeekFrom::Start(
            (file_len - bytes_to_read) as u64,
        ))
        .await?;
        reader.read_exact(&mut buffer).await?;
        for byte in buffer {
            counts[byte as usize] += 1;
        }
        bytes_read += bytes_to_read;
    }

    for &count in &counts {
        if count == 0 {
            continue;
        }

        let p: f32 = (count as f32) / (bytes_read as f32);
        entropy -= p * p.log(2.0);
    }

    Ok(entropy)
}
