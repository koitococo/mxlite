use std::{hash::Hasher, io::Read, process::Stdio};

use anyhow::Result;
use futures_util::StreamExt;
use log::{error, info};
use rand::Rng;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use xxhash_rust::xxh3::Xxh3;

/// Get the machine UUID from the DMI table.
pub(crate) fn get_machine_id() -> Result<String> {
  let mut fd = std::fs::File::open("/sys/firmware/dmi/entries/1-0/raw")?;
  let mut buf: [u8; 24] = [0u8; 24];
  fd.read_exact(&mut buf)?;
  let buf2: [u8; 16] = buf[8..24].try_into()?;
  Ok(buf2.iter().map(|b| format!("{:02x}", b)).collect())
}

pub(crate) fn random_str(len: usize) -> String {
  let mut rng = rand::rng();
  let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  (0..len).map(|_| chars.chars().nth(rng.random_range(0..chars.len())).unwrap()).collect()
}

/// Download a file from the given URL and save it to the given path. Return the xxh3 hash of the file.
pub(crate) async fn download_file(url: &str, path: &str) -> Result<String> {
  info!("Downloading file from {} to {}", url, path);
  let response = reqwest::get(url).await?;
  if response.status().is_success() {
    let mut out = File::create(path).await?;
    let mut body = response.bytes_stream();
    let mut hasher = Xxh3::new();
    while let Some(chunk) = body.next().await {
      let chunk = chunk?;
      hasher.write(&chunk);
      out.write_all(&chunk).await?;
    }
    let hash = format!("{:x}", hasher.finish());
    info!("Downloaded file from {} to {}. xxh3: {}", url, path, hash);
    Ok(hash)
  } else {
    error!("Failed to download file from {}. Server returned an error.", url);
    anyhow::bail!("Failed to download file from {}", url);
  }
}

/// Upload a file to the given URL.
pub(crate) async fn upload_file(url: &str, path: &str) -> Result<()> {
  info!("Uploading file from {} to {}", path, url);
  if reqwest::Client::new().put(url).body(File::open(path).await?).send().await?.status().is_success() {
    Ok(())
  } else {
    error!("Failed to upload file to {}. Server returned an error.", url);
    anyhow::bail!("Failed to upload file to {}", url);
  }
}

/// Execute an external command and return its output.
async fn execute_command(cmd: &String, args: Vec<String>) -> Result<(i32, String, String)> {
  info!("Executing external command: {} {:?}", cmd, args);
  let child = Command::new(cmd).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
  let output = child.wait_with_output().await?;
  Ok((
    output.status.code().unwrap_or(-1),
    String::from_utf8(output.stdout)?,
    String::from_utf8(output.stderr)?,
  ))
}

async fn execute_script(cmd: &String) -> Result<(i32, String, String)> {
  info!("Executing script: {}", cmd);
  const TMP_SCRIPT_PATH: &str = "/tmp/mxa-script.sh";
  let mut file = File::create(TMP_SCRIPT_PATH).await?;
  file.write_all(cmd.as_bytes()).await?;
  file.flush().await?;
  execute_command(&("sh".to_string()), vec![TMP_SCRIPT_PATH.to_string()]).await
}

pub(crate) async fn execute_shell(cmd: &String, use_script_file: bool) -> Result<(i32, String, String)> {
  if use_script_file {
    execute_script(cmd).await
  } else {
    execute_command(&("sh".to_string()), vec!["-c".to_string(), cmd.to_string()]).await
  }
}
