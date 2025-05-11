use std::{fs::File as std_File, hash::Hasher, io::Read, process::Stdio};

use anyhow::Result;
use futures_util::StreamExt;
use log::{error, info};
use rand::Rng;
use tokio::{fs::File, io::AsyncWriteExt, process::Command, select};
use xxhash_rust::xxh3::Xxh3;

/// Get the machine UUID from the DMI table.
pub(crate) fn get_machine_id() -> Option<String> {
  match get_machine_id_from_sysfs() {
    Ok(uuid) => return Some(uuid),
    Err(err) => {
      error!("Failed to get machine id from sysfs: {err}");
    }
  }
  match get_machine_id_from_dmi_entry() {
    Ok(uuid) => return Some(uuid),
    Err(err) => {
      error!("Failed to get machine id from dmi entry: {err}");
    }
  }
  match get_machine_id_from_dmi_table() {
    Ok(uuid) => return Some(uuid),
    Err(err) => {
      error!("Failed to get machine id from dmi table: {err}");
    }
  }
  match get_systemd_machine_id() {
    Ok(uuid) => return Some(uuid),
    Err(err) => {
      error!("Failed to get machine id from systemd: {err}");
    }
  }
  error!("Failed to get machine id from all sources");
  None
}

fn get_machine_id_from_sysfs() -> Result<String> {
  info!("Reading machine id from sysfs");
  let mut fd = std_File::open("/sys/class/dmi/id/product_uuid")?;
  let mut buf = String::new();
  fd.read_to_string(&mut buf)?;
  let uuid = buf.trim().to_string();
  Ok(uuid)
}

fn get_machine_id_from_dmi_entry() -> Result<String> {
  info!("Reading machine id from dmi entry");
  let mut fd = std_File::open("/sys/firmware/dmi/entries/1-0/raw")?;
  let mut buf = [0u8; 24];
  fd.read_exact(&mut buf)?;
  get_uuid_string_from_buf(&buf, 8)
}

fn get_machine_id_from_dmi_table() -> Result<String> {
  info!("Reading machine id from dmi table");
  let mut fd = std_File::open("/sys/firmware/dmi/tables/DMI")?;
  let mut buf = [0u8; 1024];
  if fd.read(&mut buf)? < 25 {
    anyhow::bail!("Failed to read DMI table");
  }
  let Some(offset) = buf.windows(5).position(|w| w == b"\x00\x01\x1b\x01\x00") else {
    anyhow::bail!("Failed to find entry pattern in DMI table")
  };
  get_uuid_string_from_buf(&buf, offset + 9)
}

fn get_uuid_string_from_buf(buf: &[u8], offset: usize) -> Result<String> {
  let p1 = u32::from_le_bytes(buf[offset..offset + 4].try_into()?);
  let p2 = u16::from_le_bytes(buf[offset + 4..offset + 6].try_into()?);
  let p3 = u16::from_le_bytes(buf[offset + 6..offset + 8].try_into()?);
  let p4 = u16::from_be_bytes(buf[offset + 8..offset + 10].try_into()?);
  let p5: [u8; 6] = buf[offset + 10..offset + 16].try_into()?;
  Ok(format!("{p1:08x}-{p2:04x}-{p3:04x}-{p4:04x}-") + &p5.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

fn get_systemd_machine_id() -> Result<String> {
  info!("Reading machine id from systemd");
  let mut fd = std_File::open("/etc/machine-id")?;
  let mut buf = String::new();
  fd.read_to_string(&mut buf)?;
  let uuid = buf.trim().to_string();
  Ok(format!(
    "{}-{}-{}-{}-{}",
    &uuid[0..8],
    &uuid[8..12],
    &uuid[12..16],
    &uuid[16..20],
    &uuid[20..32]
  ))
}

pub(crate) fn get_random_uuid() -> String {
  let p5: [u8; 6] = rand::random();
  format!(
    "00000000-0000-0000-0000-{}",
    &p5.iter().map(|b| format!("{b:02x}")).collect::<String>()
  )
}
pub(crate) fn random_str(len: usize) -> String {
  let mut rng = rand::rng();
  let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  (0..len).map(|_| chars.chars().nth(rng.random_range(0..chars.len())).unwrap()).collect()
}

/// Download a file from the given URL and save it to the given path. Return the xxh3 hash of the file.
pub(crate) async fn download_file(url: &str, path: &str) -> Result<String> {
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
pub(crate) async fn upload_file(url: &str, path: &str) -> Result<()> {
  info!("Uploading file from {path} to {url}");
  if reqwest::Client::new().put(url).body(File::open(path).await?).send().await?.status().is_success() {
    Ok(())
  } else {
    error!("Failed to upload file to {url}. Server returned an error.");
    anyhow::bail!("Failed to upload file to {}", url);
  }
}

/// Execute an external command and return its output.
async fn execute_command(cmd: &String, args: Vec<String>) -> Result<(i32, String, String)> {
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

async fn execute_script(cmd: &String) -> Result<(i32, String, String)> {
  info!("Executing script: {cmd}");
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

pub(crate) async fn safe_sleep(duration: u64) -> bool {
  select! {
      _ = tokio::time::sleep(std::time::Duration::from_millis(duration)) => {
          false
      },
      _ = tokio::signal::ctrl_c() => {
          info!("Received Ctrl-C, shutting down");
          true
      }
  }
}
