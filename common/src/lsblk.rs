use std::{
    fs::read_dir,
    io::{self, Read},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlkInfo {
    pub maj_min: String,             // major:minor number, e.g., 8:0, 8:1
    pub disk_seq: u64,               // disk sequence number, e.g., 0, 1
    pub name: String,                // device name, e.g., sda, sdb
    pub kname: String,               // kernel blk name, e.g., sda, sdb
    pub model: Option<String>,       // device model, e.g., WDS500G3X0C-00SJG0
    pub size: u64,                   // device size in bytes
    pub removable: bool,             // whether the device is removable
    pub uuid: Option<String>,        // UUID of the device
    pub wwid: Option<String>,        // WWID of the device
    pub readonly: bool,              // whether the device is read-only
    pub path: Option<String>,        // device path, e.g., /dev/sda
    pub path_by_seq: Option<String>, // device path by sequence number, e.g., /dev/disk/by-seq/0
    pub subsystem: Option<String>,   // subsystem of the device, e.g., nvme, scsi
}

fn lsdir(path: &str) -> Result<Vec<String>, io::Error> {
    Ok(read_dir(path)?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.file_name().to_string_lossy().to_string()),
            Err(_) => None,
        })
        .collect())
}

fn read_str(path: &str) -> Result<String, io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
}

fn read_str_optional(path: &str) -> Result<Option<String>, io::Error> {
    match read_str(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn read_int(path: &str) -> Result<u64, io::Error> {
    let content = read_str(path)?;
    match content.parse::<u64>() {
        Ok(num) => Ok(num),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse integer from {}: {}", path, e),
        )),
    }
}

fn read_bool(path: &str) -> Result<bool, io::Error> {
    Ok(read_int(path)? != 0)
}

fn read_list(path: &str) -> Result<Vec<String>, io::Error> {
    let contents = read_str(path)?;
    Ok(contents
        .trim()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect())
}

fn read_kv(path: &str) -> Result<Vec<(String, String)>, io::Error> {
    read_list(path).map(|list| {
        list.into_iter()
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

fn checked_path(path: &str) -> Result<String, io::Error> {
    if std::fs::exists(path)? {
        Ok(path.to_string())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path {} does not exist", path),
        ))
    }
}

fn read_symlink(path: &str) -> Result<String, io::Error> {
    let symlink = std::fs::canonicalize(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read symlink {}: {}", path, e),
        )
    })?;
    Ok(symlink.to_string_lossy().to_string())
}

fn create_root_blk_info(blk_name: &str) -> Result<BlkInfo, io::Error> {
    let maj_min = read_str(format!("/sys/block/{}/dev", blk_name).as_str())?;
    let disk_seq = read_int(format!("/sys/block/{}/diskseq", blk_name).as_str())?;

    Ok(BlkInfo {
        maj_min,
        disk_seq,
        name: read_kv(format!("/sys/block/{}/uevent", blk_name).as_str())?
            .iter()
            .find(|(k, _)| k == "DEVNAME")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default(),
        kname: blk_name.to_string(),
        model: read_str_optional(format!("/sys/block/{}/device/model", blk_name).as_str())?,
        size: read_int(format!("/sys/block/{}/size", blk_name).as_str()).map(|sectors| {
            read_int(format!("/sys/block/{}/queue/logical_block_size", blk_name).as_str())
                .map(|block_size| sectors * block_size)
                .unwrap_or(sectors * 512)
        })?,
        removable: read_bool(format!("/sys/block/{}/removable", blk_name).as_str())?,
        uuid: read_str_optional(format!("/sys/block/{}/uuid", blk_name).as_str())?,
        wwid: read_str_optional(format!("/sys/block/{}/wwid", blk_name).as_str())?,
        readonly: read_bool(format!("/sys/block/{}/ro", blk_name).as_str())?,
        path: checked_path(format!("/dev/{}", blk_name).as_str()).ok(),
        path_by_seq: checked_path(format!("/dev/disk/by-diskseq/{}", disk_seq).as_str()).ok(),
        subsystem: read_symlink(format!("/sys/block/{}/device/subsystem", blk_name).as_str())
            .ok()
            .and_then(|s| match s.as_str() {
                "/sys/class/nvme" => Some("nvme".to_string()),
                "/sys/bus/scsi" => Some("scsi".to_string()),
                _ => Some("unknown".to_string()),
            }),
    })
}

fn ls_root_blks() -> Result<Vec<String>, io::Error> {
    Ok(lsdir("/sys/block")?)
}

pub fn get_blk_info() -> Vec<BlkInfo> {
    ls_root_blks()
        .map(|blks| {
            blks.iter()
                .filter_map(|blk_name| create_root_blk_info(blk_name).ok())
                .collect()
        })
        .unwrap_or(vec![])
}
