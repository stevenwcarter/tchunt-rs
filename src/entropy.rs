use anyhow::Result;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader};

pub struct Entropy<'a, R> {
    source: &'a mut R,
    reader: BufReader<R>,
    length: usize,
}

impl<'a> Entropy<'a, tokio::fs::File> {
    pub async fn new_from_file(file: &'a mut tokio::fs::File) -> Result<Self> {
        let length: usize = file.metadata().await?.len() as usize;
        let reader = BufReader::new(file.try_clone().await?);

        Ok(Self {
            source: file,
            reader,
            length,
        })
    }
}
impl<'a> Entropy<'a, Cursor<Vec<u8>>> {
    pub async fn new_from_vec(cursor: &'a mut Cursor<Vec<u8>>, length: usize) -> Result<Self> {
        let reader = BufReader::new(cursor.clone());

        Ok(Self {
            source: cursor,
            reader,
            length,
        })
    }
}

impl<'a, R> Entropy<'a, R> {
    pub async fn shannon(&mut self) -> Result<f32>
    where
        R: AsyncReadExt + AsyncSeekExt + Unpin,
    {
        let mut entropy = 0.0;
        let mut counts = [0usize; 256];

        let mut bytes_to_read: usize = 171072;
        let file_len: usize = self.length;
        if file_len < bytes_to_read {
            bytes_to_read = file_len;
        }
        let mut bytes_read = bytes_to_read;

        let mut buffer = vec![0; bytes_to_read];
        self.reader.read_exact(&mut buffer).await?;

        for byte in buffer {
            counts[byte as usize] += 1;
        }
        if file_len > 2 * bytes_to_read {
            let mut buffer = vec![0; bytes_to_read];
            self.source
                .seek(tokio::io::SeekFrom::Start(
                    (file_len - bytes_to_read) as u64,
                ))
                .await?;
            self.reader.read_exact(&mut buffer).await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_calculates_entropy_correctly() {
        let vec = vec![0; 200];
        let len = vec.len();
        let mut cursor = Cursor::new(vec);

        let mut entropy = Entropy::new_from_vec(&mut cursor, len).await.unwrap();

        assert_eq!(entropy.shannon().await.unwrap(), 0.0);
    }
    #[tokio::test]
    async fn it_calculates_entropy_for_high_entropy_sources() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let vec: Vec<u8> = (0..2000000).map(|_| rng.gen_range(0..255)).collect();
        let len = vec.len();
        let mut cursor = Cursor::new(vec);

        let mut entropy = Entropy::new_from_vec(&mut cursor, len).await.unwrap();

        assert!(entropy.shannon().await.unwrap() > 7.8);
    }
}
