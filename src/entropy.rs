use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};

/// Number of bytes sampled from the head (and tail) of a file for entropy calculation.
/// 171072 = 167 × 1024, chosen to balance accuracy against I/O cost on large files.
const SAMPLE_SIZE: usize = 171_072;

pub struct Entropy<R> {
    reader: BufReader<R>,
    length: usize,
    head: Vec<u8>,
}

impl<R: AsyncReadExt + AsyncSeekExt + Unpin> Entropy<R> {
    pub fn new(reader: R, length: usize) -> Self {
        Self {
            reader: BufReader::new(reader),
            length,
            head: Vec::new(),
        }
    }
}

impl<R> Entropy<R> {
    /// Returns the bytes read from the head of the file during `shannon()`.
    /// Empty before `shannon()` is called.
    pub fn head_bytes(&self) -> &[u8] {
        &self.head
    }

    pub async fn shannon(&mut self) -> Result<f64>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin,
    {
        let mut entropy = 0.0f64;
        let mut counts = [0usize; 256];

        let file_len: usize = self.length;
        let bytes_to_read: usize = SAMPLE_SIZE.min(file_len);
        let mut bytes_read = bytes_to_read;

        let mut tally = |buf: &[u8]| {
            for byte in buf {
                counts[*byte as usize] += 1;
            }
        };

        let mut buffer = vec![0u8; bytes_to_read];
        self.reader.read_exact(&mut buffer).await?;
        tally(&buffer);
        self.head = buffer;

        let tail_start = file_len.saturating_sub(bytes_to_read);
        if tail_start > bytes_to_read {
            self.reader
                .seek(tokio::io::SeekFrom::Start(tail_start as u64))
                .await?;
            let mut tail = vec![0u8; bytes_to_read];
            self.reader.read_exact(&mut tail).await?;
            tally(&tail);
            bytes_read += bytes_to_read;
        }

        for &count in &counts {
            if count == 0 {
                continue;
            }
            let p: f64 = (count as f64) / (bytes_read as f64);
            entropy -= p * p.log2();
        }

        Ok(entropy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn it_calculates_entropy_correctly() {
        let vec = vec![0u8; 200];
        let len = vec.len();
        let cursor = Cursor::new(vec);

        let mut entropy = Entropy::new(cursor, len);

        assert_eq!(entropy.shannon().await.unwrap(), 0.0f64);
    }

    #[tokio::test]
    async fn it_calculates_entropy_for_high_entropy_sources() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let vec: Vec<u8> = (0..2000000).map(|_| rng.gen_range(0..=255)).collect();
        let len = vec.len();
        let cursor = Cursor::new(vec);

        let mut entropy = Entropy::new(cursor, len);

        assert!(entropy.shannon().await.unwrap() > 7.8);
    }

    /// Exercises the two-sample path: first half all 0x00, second half all 0xFF.
    /// Two equally-weighted symbols → entropy = 1.0 bit.
    /// With the BufReader aliasing bug, this would return near 0.0 instead.
    #[tokio::test]
    async fn it_calculates_entropy_two_sample_path() {
        let size = 4 * SAMPLE_SIZE;
        let mut vec = vec![0x00u8; size / 2];
        vec.extend(vec![0xFFu8; size / 2]);
        let cursor = Cursor::new(vec);

        let mut entropy = Entropy::new(cursor, size);
        let result = entropy.shannon().await.unwrap();

        // Two equally-weighted symbols → exactly 1.0 bit of entropy
        assert!((result - 1.0).abs() < 1e-9, "Expected ~1.0, got {result}");
    }

    /// Exercises Entropy::new with the random65536 test fixture.
    #[tokio::test]
    async fn it_calculates_entropy_from_file() {
        let file = tokio::fs::File::open("test-resources/random65536")
            .await
            .expect("test-resources/random65536 must exist");
        let length = file.metadata().await.unwrap().len() as usize;
        let mut entropy = Entropy::new(file, length);

        assert!(
            entropy.shannon().await.unwrap() > 7.0,
            "random data should have high entropy"
        );
    }
}
