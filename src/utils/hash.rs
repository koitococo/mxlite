use std::{hash::Hasher, io::Error};
use thiserror::Error;

use base16ct::lower;
use digest::{Digest, DynDigest};
use tokio::{fs::File, io::AsyncReadExt};
use xxhash_rust::xxh3::Xxh3;

/// Calculate hash for a file at the given path. Uses `std::hash::Hasher` trait.
///
/// If the file is smaller than 1MB, it reads the whole file into memory.
///
/// Returns the hash in base16 format.
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

/// Calculate xxh3 hash for a file at the given path.
///
/// Returns the hash in base16 format.
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

/// Calculate a digest for a file at the given path using the provided hasher. Uses `digest::Digest` trait.
///
/// If the file is smaller than 1MB, it reads the whole file into memory.
///
/// Returns the hash in base16 format.
pub async fn digest_for_file(path: &str, mut hasher: Box<dyn DynDigest + Send + Unpin>) -> Result<String, HashError> {
  let mut fd = File::open(path).await?;
  // for files smaller than 1MB we can read the whole file into memory
  let metadata = fd.metadata().await.map_err(HashError::IoError)?;
  if metadata.len() < 1024 * 1024 {
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

/// Calculate MD5 hash for a file at the given path.
///
/// Returns the hash in base16 format.
pub async fn md5_for_file(path: &str) -> Result<String, HashError> {
  digest_for_file(path, Box::new(<md5::Md5 as Digest>::new())).await
}

/// Calculate SHA1 hash for a file at the given path.
///
/// Returns the hash in base16 format.
pub async fn sha1_for_file(path: &str) -> Result<String, HashError> {
  digest_for_file(path, Box::new(<sha1::Sha1 as Digest>::new())).await
}

/// Calculate SHA2-256 hash for a file at the given path.
///
/// Returns the hash in base16 format.
pub async fn sha2_256_for_file(path: &str) -> Result<String, HashError> {
  digest_for_file(path, Box::new(<sha2::Sha256 as Digest>::new())).await
}

/// Calculate SHA3-512 hash for a file at the given path.
///
/// Returns the hash in base16 format.
pub async fn sha3_512_for_file(path: &str) -> Result<String, HashError> {
  digest_for_file(path, Box::new(<sha3::Sha3_512 as Digest>::new())).await
}

/// Calculate digests for a file at the given path using the provided hashers.
///
/// Returns a vector of hashes in base16 format.
pub async fn digests_for_file(
  path: &str, mut hashers: Vec<Box<dyn DynDigest + Send + Unpin>>,
) -> Result<Vec<String>, HashError> {
  let mut fd = File::open(path).await?;
  // for files smaller than 1MB we can read the whole file into memory
  let metadata = fd.metadata().await.map_err(HashError::IoError)?;
  if metadata.len() < 1024 * 1024 {
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

/// Calculate multiple hashes for a file at the given path.
pub async fn hashes_for_file(
  path: &str, calc_md5: bool, calc_sha1: bool, calc_sha2_256: bool, calc_sha3_512: bool,
) -> Result<(Option<String>, Option<String>, Option<String>, Option<String>), HashError> {
  let mut hashers: Vec<Box<dyn DynDigest + Send + Unpin>> = Vec::new();
  if calc_md5 {
    hashers.push(Box::new(<md5::Md5 as Digest>::new()));
  }
  if calc_sha1 {
    hashers.push(Box::new(<sha1::Sha1 as Digest>::new()));
  }
  if calc_sha2_256 {
    hashers.push(Box::new(<sha2::Sha256 as Digest>::new()));
  }
  if calc_sha3_512 {
    hashers.push(Box::new(<sha3::Sha3_512 as Digest>::new()));
  }
  let mut hashes = digests_for_file(path, hashers).await?;
  let mut result: (Option<String>, Option<String>, Option<String>, Option<String>) = (None, None, None, None);
  if calc_md5 {
    let hash = hashes.remove(0);
    result.0 = Some(hash);
  }
  if calc_sha1 {
    let hash = hashes.remove(0);
    result.1 = Some(hash);
  }
  if calc_sha2_256 {
    let hash = hashes.remove(0);
    result.2 = Some(hash);
  }
  if calc_sha3_512 {
    let hash = hashes.remove(0);
    result.3 = Some(hash);
  }
  Ok(result)
}

pub fn sha2_256_for_str(input: &str) -> Result<String, HashError> {
  let mut hasher = sha2::Sha256::new();
  Digest::update(&mut hasher, input.as_bytes());
  let hash = hasher.finalize();
  let mut buf = vec![0u8; hash.len() * 2];
  Ok(lower::encode_str(&hash, buf.as_mut_slice()).map_err(HashError::Base16Error)?.to_string())
}
