use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};

pub async fn shannon(file: &mut tokio::fs::File) -> Result<f32> {
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
