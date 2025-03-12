use std::{hash::Hasher, io::Error};

use tokio::{fs::File, io::AsyncReadExt};
use xxhash_rust::xxh3::{self, Xxh3};

pub async fn xxh3_for_file(path: &str) -> Result<String, Error> {
    let mut fd = File::open(path).await?;
    // for files smaller than 1MB we can read the whole file into memory
    if fd.metadata().await?.len() < 1024 * 1024 {
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf).await?;
        return Ok(format!("{:x}", xxh3::xxh3_64(&buf)));
    }
    let mut hasher = Xxh3::new();
    let mut buf = [0u8; 4 * 1024];
    loop {
        let n = fd.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finish()))
}
