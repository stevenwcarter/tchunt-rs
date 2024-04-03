use anyhow::{Context, Result};
use async_walkdir::WalkDir;
use futures::executor::block_on;
use futures_lite::stream::StreamExt;
use log::*;
use std::sync::Arc;
use std::time::Instant;
use std::{env, thread};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = env::args().collect();

    let tasks = Arc::new(Mutex::new(JoinSet::new()));

    let finished = Arc::new(Mutex::new(false));
    let thread_finished = finished.clone();

    let handle = Arc::new(Handle::current());
    let thread_handle = handle.clone();
    // let (tx, mut rx) = mpsc::channel(100);
    let thread_tasks = tasks.clone();
    let search_thread = thread::spawn(move || {
        thread_handle.block_on(async move {
            let mut counter = 0;
            let start = Instant::now();
            let mut entries = WalkDir::new(&args[1]);
            loop {
                match entries.next().await {
                    Some(Ok(entry)) => {
                        counter += 1;
                        let metadata =
                            tokio::fs::metadata(entry.path().display().to_string()).await;
                        if let Ok(metadata) = metadata {
                            let length = metadata.len();
                            if length % 512 == 0 && length > 1024 * 1024 * 30 {
                                // println!("- {filename}");
                                thread_tasks.lock().await.spawn(async move {
                                    let _ = check_file(&entry.path().display().to_string()).await;
                                });
                            }
                        }
                    }
                    Some(Err(_)) => {
                        // eprintln!("error: {}", e);
                    }
                    None => break,
                }
            }
            let duration = start.elapsed();
            info!("Time taken to find entries: {:?}", duration);
            info!("Checked {counter} files");
            *thread_finished.lock().await = true;
        })
    });

    let executor = thread::spawn(move || {
        let start = Instant::now();
        block_on(async move {
            while !*finished.lock().await {
                while tasks.lock().await.join_next().await.is_some() {
                    //
                }
            }
        });
        let duration = start.elapsed();
        info!("Time taken to scan entries: {:?}", duration);
    });

    search_thread.join().unwrap();
    executor.join().unwrap();

    Ok(())
}
async fn check_file(filename: &str) -> Result<()> {
    let metadata = tokio::fs::metadata(filename)
        .await
        .context("Could not open file")?;
    let length = metadata.len();
    if length % 512 != 0 || length < 512 {
        // println!("- {filename}");
        return Ok(());
    }
    let mut file = File::open(filename)
        .await
        .context("Could not open file for tokio")?;
    // let mut contents = vec![];
    // file.read_to_end(&mut contents).await?;

    let shannon = shannon_entropy(&mut file)
        .await
        .context("could not get shannon entropy")?;
    // println!("shannon entropy: {shannon}");
    if shannon < 7.9 {
        // println!("- {filename}");
        return Ok(());
    }

    println!("{shannon} {filename}");

    Ok(())
}

async fn shannon_entropy(file: &mut tokio::fs::File) -> Result<f32> {
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

    // for &b in bytes {
    //     counts[b as usize] += 1;
    // }

    for &count in &counts {
        if count == 0 {
            continue;
        }

        let p: f32 = (count as f32) / (bytes_read as f32);
        entropy -= p * p.log(2.0);
    }

    // println!("Entropy: {entropy}");

    Ok(entropy)
}
