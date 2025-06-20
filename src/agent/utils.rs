use std::{fs::File as std_File, io::Read};

use anyhow::Result;
use log::{error, info};
use tokio::select;

/// Get the machine UUID from the DMI table.
/// **Only works on Linux**
pub(crate) fn get_machine_id() -> Option<String> {
  #[cfg(target_os = "linux")]
  {
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
  }
  None
}

#[cfg(target_os = "linux")]
fn get_machine_id_from_sysfs() -> Result<String> {
  info!("Reading machine id from sysfs");
  let mut fd = std_File::open("/sys/class/dmi/id/product_uuid")?;
  let mut buf = String::new();
  fd.read_to_string(&mut buf)?;
  let uuid = buf.trim().to_string();
  Ok(uuid)
}

#[cfg(target_os = "linux")]
fn get_machine_id_from_dmi_entry() -> Result<String> {
  info!("Reading machine id from dmi entry");
  let mut fd = std_File::open("/sys/firmware/dmi/entries/1-0/raw")?;
  let mut buf = [0u8; 24];
  fd.read_exact(&mut buf)?;
  get_uuid_string_from_buf(&buf, 8)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
