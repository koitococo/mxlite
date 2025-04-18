use std::{hash::Hasher, io::Error};
use thiserror::Error;

use base16ct::lower;
use digest::DynDigest;
use tokio::{fs::File, io::AsyncReadExt};
use xxhash_rust::xxh3::Xxh3;

pub async fn hasher_for_file<H: Hasher>(path: &str, hasher: &mut H) -> Result<String, Error> {
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
  hasher_for_file(path, &mut hasher).await
}

#[derive(Debug, Error)]
pub enum HashError {
  #[error("Base16 encoding error: {0}")]
  Base16Error(base16ct::Error),
  #[error("IO error: {0}")]
  IoError(#[from] std::io::Error),
}

pub async fn digest_for_file(path: &str, mut hasher: Box<dyn DynDigest + Send + Unpin>) -> Result<String, HashError> {
  let mut fd = File::open(path).await?;
  // for files smaller than 1MB we can read the whole file into memory
  if fd.metadata().await.unwrap().len() < 1024 * 1024 {
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf).await?;
    hasher.update(&buf);
  } else {
    let mut buf = [0u8; 4 * 1024];
    loop {
      let n = fd.read(&mut buf).await?;
      if n == 0 {
        break;
      }
      hasher.update(&buf[..n]);
    }
  }
  let hash = hasher.finalize_reset();
  let mut buf = vec![0u8; hash.len() * 2];
  Ok(lower::encode_str(&hash, buf.as_mut_slice()).map_err(HashError::Base16Error)?.to_string())
}

/// MD5 hash for a file
pub async fn md5_for_file(path: &str) -> Result<String, HashError> { digest_for_file(path, Box::new(<md5::Md5 as digest::Digest>::new())).await }

/// SHA1 hash for a file
pub async fn sha1_for_file(path: &str) -> Result<String, HashError> { digest_for_file(path, Box::new(<sha1::Sha1 as digest::Digest>::new())).await }

/// SHA2-256 hash for a file
pub async fn sha256_for_file(path: &str) -> Result<String, HashError> { digest_for_file(path, Box::new(<sha2::Sha256 as digest::Digest>::new())).await }

/// SHA3-512 hash for a file
pub async fn sha512_for_file(path: &str) -> Result<String, HashError> { digest_for_file(path, Box::new(<sha3::Sha3_512 as digest::Digest>::new())).await }

pub async fn digests_for_file(path: &str, mut hashers: Vec<Box<dyn DynDigest + Send + Unpin>>) -> Result<Vec<String>, HashError> {
  let mut fd = File::open(path).await?;
  // for files smaller than 1MB we can read the whole file into memory
  if fd.metadata().await.unwrap().len() < 1024 * 1024 {
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf).await?;
    for hasher in &mut hashers {
      hasher.update(&buf);
    }
  } else {
    let mut buf = [0u8; 4 * 1024];
    loop {
      let n = fd.read(&mut buf).await?;
      if n == 0 {
        break;
      }
      for hasher in &mut hashers {
        hasher.update(&buf[..n]);
      }
    }
  }

  let mut hashes = Vec::with_capacity(hashers.len());
  for hasher in &mut hashers {
    let hash = hasher.finalize_reset();
    let mut buf = vec![0u8; hash.len() * 2];
    hashes.push(lower::encode_str(&hash, buf.as_mut_slice()).map_err(HashError::Base16Error)?.to_string());
  }
  Ok(hashes)
}

pub async fn sha_for_file(
  path: &str, calc_md5: bool, calc_sha1: bool, calc_sha256: bool, calc_sha512: bool,
) -> Result<(Option<String>, Option<String>, Option<String>, Option<String>), HashError> {
  let mut hashers: Vec<Box<dyn DynDigest + Send + Unpin>> = Vec::new();
  if calc_md5 {
    hashers.push(Box::new(<md5::Md5 as md5::Digest>::new()));
  }
  if calc_sha1 {
    hashers.push(Box::new(<sha1::Sha1 as sha1::Digest>::new()));
  }
  if calc_sha256 {
    hashers.push(Box::new(<sha2::Sha256 as sha2::Digest>::new()));
  }
  if calc_sha512 {
    hashers.push(Box::new(<sha3::Sha3_512 as sha3::Digest>::new()));
  }
  let mut hashes = digests_for_file(path, hashers).await?;
  let mut result: (Option<String>, Option<String>, Option<String>, Option<String>) = (None, None, None, None);
  if calc_md5 {
    let hash = hashes.remove(0);
    result.0 = Some(hash);
  }
  if calc_sha1 {
    let hash = hashes.remove(0);
    result.0 = Some(hash);
  }
  if calc_sha256 {
    let hash = hashes.remove(0);
    result.1 = Some(hash);
  }
  if calc_sha512 {
    let hash = hashes.remove(0);
    result.2 = Some(hash);
  }
  Ok(result)
}
