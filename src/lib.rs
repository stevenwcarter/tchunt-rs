use anyhow::{Context, Result};
use async_walkdir::{Filtering, WalkDir};
use futures::executor::block_on;
use futures_lite::stream::StreamExt;
use log::*;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

pub async fn search_dir(directory: &'static str) -> Result<()> {
    let tasks = Arc::new(Mutex::new(JoinSet::new()));

    let finished = Arc::new(Mutex::new(false));
    let thread_finished = finished.clone();

    let handle = Arc::new(tokio::runtime::Handle::current());
    let thread_handle = handle.clone();

    let (tx, mut rx) = mpsc::channel(100);
    let thread_tasks = tasks.clone();
    let search_thread = tokio::task::spawn_blocking(move || {
        thread_handle.block_on(async move {
            let mut counter = 0;
            let start = std::time::Instant::now();
            let mut entries = WalkDir::new(directory).filter(|entry| async move {
                if let Ok(metadata) = tokio::fs::metadata(entry.path().display().to_string()).await
                {
                    if metadata.is_file() {
                        let length = metadata.len();
                        if length % 512 == 0 && length > 1024 * 1024 * 30 {
                            return Filtering::Continue;
                        } else {
                            return Filtering::Ignore;
                        }
                    }
                }
                Filtering::Continue
            });

            while let Some(Ok(entry)) = entries.next().await {
                counter += 1;
                let tx = tx.clone();
                let entry_path = entry.path().display().to_string();
                thread_tasks.lock().await.spawn(async move {
                    let _ = check_file(&entry_path).await;
                    tx.send(()).await.expect("Failed to send signal");
                });
            }

            let duration = start.elapsed();
            info!("Time taken to find entries: {:?}", duration);
            info!("Checked {} files", counter);
            *thread_finished.lock().await = true;
        })
    });

    let executor = tokio::task::spawn(async move {
        let start = std::time::Instant::now();
        while !*finished.lock().await {
            rx.recv().await;
            while tasks.lock().await.join_next().await.is_some() {
                // Do nothing
            }
        }
        let duration = start.elapsed();
        info!("Time taken to scan entries: {:?}", duration);
    });

    search_thread.await.expect("Search thread panicked");
    executor.await.expect("Executor thread panicked");

    Ok(())
}

pub async fn check_file(filename: &str) -> Result<()> {
    let metadata = tokio::fs::metadata(filename)
        .await
        .context("Could not open file")?;
    let length = metadata.len();
    if length % 512 != 0 || length < 512 {
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

    println!("{} {}", shannon, filename);

    Ok(())
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
