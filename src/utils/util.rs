use std::{hash::Hasher, process::Stdio};

use anyhow::Result;
use futures_util::StreamExt;
use log::{error, info};
use rand::Rng;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use xxhash_rust::xxh3::Xxh3;

/// Download a file from the given URL and save it to the given path. Return the xxh3 hash of the file.
pub async fn download_file(url: &str, path: &str) -> Result<String> {
  info!("Downloading file from {url} to {path}");
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
    info!("Downloaded file from {url} to {path}. xxh3: {hash}");
    Ok(hash)
  } else {
    error!("Failed to download file from {url}. Server returned an error.");
    anyhow::bail!("Failed to download file from {}", url);
  }
}

/// Upload a file to the given URL.
pub async fn upload_file(url: &str, path: &str) -> Result<()> {
  info!("Uploading file from {path} to {url}");
  if reqwest::Client::new().put(url).body(File::open(path).await?).send().await?.status().is_success() {
    Ok(())
  } else {
    error!("Failed to upload file to {url}. Server returned an error.");
    anyhow::bail!("Failed to upload file to {}", url);
  }
}

/// Execute an external command and return its output.
pub async fn execute_command(cmd: &String, args: Vec<String>) -> Result<(i32, String, String)> {
  info!("Executing external command: {cmd} {args:?}");
  let child = Command::new(cmd)
    .args(args)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
  let output = child.wait_with_output().await?;
  Ok((
    output.status.code().unwrap_or(-1),
    String::from_utf8(output.stdout)?,
    String::from_utf8(output.stderr)?,
  ))
}

#[inline]
async fn execute_script(cmd: &String) -> Result<(i32, String, String)> {
  info!("Executing script: {cmd}");
  // FIXME: should not use a hardcoded path
  const TMP_SCRIPT_PATH: &str = "/tmp/mxa-script.sh";
  let mut file = File::create(TMP_SCRIPT_PATH).await?;
  file.write_all(cmd.as_bytes()).await?;
  file.flush().await?;
  execute_command(&("sh".to_string()), vec![TMP_SCRIPT_PATH.to_string()]).await
}

/// Execute a shell command or script file with `sh`.
/// 
/// On most Linux distributions, the `sh` command is a symlink to `bash`.
/// On macOS, it is a symlink to `bash` 3.0 version.
/// **Should NOT work on Windows**
pub async fn execute_shell(cmd: &String, use_script_file: bool) -> Result<(i32, String, String)> {
  if use_script_file {
    execute_script(cmd).await
  } else {
    execute_command(&("sh".to_string()), vec!["-c".to_string(), cmd.to_string()]).await
  }
}

/// Generate a random UUID in the format `00000000-0000-0000-0000-xxxxxxxxxxxx` where `x` is a random hex digit.
pub fn get_random_uuid() -> String {
  let p5: [u8; 6] = rand::random();
  format!(
    "00000000-0000-0000-0000-{}",
    &p5.iter().map(|b| format!("{b:02x}")).collect::<String>()
  )
}

/// Generate a random string of the given length using alphanumeric characters.
pub fn random_str(len: usize) -> String {
  let mut rng = rand::rng();
  let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  (0..len).map(|_| chars.chars().nth(rng.random_range(0..chars.len())).unwrap()).collect()
}
