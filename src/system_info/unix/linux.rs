#![cfg(target_os = "linux")]

use super::BlkInfo;
use std::{
  fs::read_dir,
  io::{self, Read},
};

pub(super) fn get_blk_info() -> Vec<BlkInfo> {
  #[inline]
  fn lsdir(path: &str) -> Result<Vec<String>, io::Error> {
    Ok(
      read_dir(path)?
        .filter_map(|entry| match entry {
          Ok(entry) => Some(entry.file_name().to_string_lossy().to_string()),
          Err(_) => None,
        })
        .collect(),
    )
  }

  #[inline]
  fn read_str(path: &str) -> Result<String, io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
  }

  #[inline]
  fn read_str_optional(path: &str) -> Result<Option<String>, io::Error> {
    match read_str(path) {
      Ok(contents) => Ok(Some(contents)),
      Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(e) => Err(e),
    }
  }

  #[inline]
  fn read_int(path: &str) -> Result<u64, io::Error> {
    let content = read_str(path)?;
    match content.parse::<u64>() {
      Ok(num) => Ok(num),
      Err(e) => Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Failed to parse integer from {path}: {e}"),
      )),
    }
  }

  #[inline]
  fn read_bool(path: &str) -> Result<bool, io::Error> { Ok(read_int(path)? != 0) }

  #[inline]
  fn read_list(path: &str) -> Result<Vec<String>, io::Error> {
    let contents = read_str(path)?;
    Ok(contents.split_whitespace().map(|s| s.to_string()).collect())
  }

  #[inline]
  fn read_kv(path: &str) -> Result<Vec<(String, String)>, io::Error> {
    read_list(path).map(|list| {
      list
        .into_iter()
        .filter_map(|s| {
          let mut parts = s.split('=');
          if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            Some((key.to_string(), value.to_string()))
          } else {
            None
          }
        })
        .collect()
    })
  }

  #[inline]
  fn checked_path(path: &str) -> Result<String, io::Error> {
    if std::fs::exists(path)? {
      Ok(path.to_string())
    } else {
      Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Path {path} does not exist"),
      ))
    }
  }

  #[inline]
  fn read_symlink(path: &str) -> Result<String, io::Error> {
    let symlink = std::fs::canonicalize(path)
      .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Failed to read symlink {path}: {e}")))?;
    Ok(symlink.to_string_lossy().to_string())
  }

  #[inline]
  fn create_root_blk_info(blk_name: &str) -> Result<BlkInfo, io::Error> {
    let maj_min = read_str(format!("/sys/block/{blk_name}/dev").as_str())?;
    let disk_seq = read_int(format!("/sys/block/{blk_name}/diskseq").as_str())?;

    Ok(BlkInfo {
      maj_min,
      disk_seq,
      name: read_kv(format!("/sys/block/{blk_name}/uevent").as_str())?
        .iter()
        .find(|(k, _)| k == "DEVNAME")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default(),
      kname: blk_name.to_string(),
      model: read_str_optional(format!("/sys/block/{blk_name}/device/model").as_str())?,
      size: read_int(format!("/sys/block/{blk_name}/size").as_str()).map(|sectors| {
        sectors << 9 // 512 bytes per sector
      })?,
      removable: read_bool(format!("/sys/block/{blk_name}/removable").as_str())?,
      uuid: read_str_optional(format!("/sys/block/{blk_name}/uuid").as_str())?,
      wwid: read_str_optional(format!("/sys/block/{blk_name}/wwid").as_str())?,
      readonly: read_bool(format!("/sys/block/{blk_name}/ro").as_str())?,
      path: checked_path(format!("/dev/{blk_name}").as_str()).ok(),
      path_by_seq: checked_path(format!("/dev/disk/by-diskseq/{disk_seq}").as_str()).ok(),
      subsystem: read_symlink(format!("/sys/block/{blk_name}/device/subsystem").as_str()).ok().map(|s| {
        match s.as_str() {
          "/sys/class/nvme" => "nvme".to_string(),
          "/sys/bus/scsi" => "scsi".to_string(),
          _ => "unknown".to_string(),
        }
      }),
    })
  }

  #[inline]
  fn ls_root_blks() -> Result<Vec<String>, io::Error> { lsdir("/sys/block") }

  ls_root_blks()
    .map(|blks| blks.iter().filter_map(|blk_name| create_root_blk_info(blk_name).ok()).collect())
    .unwrap_or_default()
}
