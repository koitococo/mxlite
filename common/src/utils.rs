use std::{hash::Hasher, io::Error};
use thiserror::Error;

use base16ct::lower;
use sha1::digest::DynDigest;
use tokio::{fs::File, io::AsyncReadExt};
use xxhash_rust::xxh3::Xxh3;

pub async fn hash_for_file<H: Hasher>(path: &str, hasher: &mut H) -> Result<String, Error> {
    let mut fd = File::open(path).await?;
    // for files smaller than 1MB we can read the whole file into memory
    if fd.metadata().await?.len() < 1024 * 1024 {
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf).await?;
        hasher.write(&buf);
        return Ok(format!("{:x}", hasher.finish()));
    }
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

pub async fn xxh3_for_file(path: &str) -> Result<String, Error> {
    let mut hasher = Xxh3::new();
    hash_for_file(path, &mut hasher).await
}

#[derive(Debug, Error)]
pub enum HashError {
    #[error("Base16 encoding error: {0}")]
    Base16Error(base16ct::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub async fn hash_for_file2<F>(path: &str, get_hasher: F) -> Result<String, HashError>
where
    F: FnOnce() -> Box<dyn DynDigest + Sync + Send + Unpin>,
{
    let mut fd = File::open(path).await?;
    // for files smaller than 1MB we can read the whole file into memory
    let mut hasher = get_hasher();
    if fd.metadata().await.unwrap().len() < 1024 * 1024 {
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf).await?;
        hasher.update(&buf);
    }
    let mut buf = [0u8; 4 * 1024];
    loop {
        let n = fd.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize_reset();
    let mut buf = Vec::with_capacity(hash.len() * 2);
    let r = lower::encode_str(&hash, buf.as_mut_slice()).map_err(HashError::Base16Error)?;
    Ok(r.to_string())
}

pub async fn sha1_for_file(path: &str) -> Result<String, HashError> {
    hash_for_file2(path, || Box::new(<sha1::Sha1 as sha1::Digest>::new())).await
}
