use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
  pub names: Vec<String>,
  pub vendor_id: String,
  pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MntInfo {
  pub kind: String,
  pub device_name: String,
  pub file_system: String,
  pub mount_point: String,
  pub total_space: u64,
  pub is_removable: bool,
  pub is_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpInfo {
  pub addr: String,
  pub version: u8,
  pub prefix: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NicInfo {
  pub mac_address: String,
  pub mtu: u64,
  pub ip: Vec<IpInfo>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtsInfo {
  pub sysname: String,
  pub nodename: String,
  pub release: String,
  pub version: String,
  pub machine: String,
  pub domainname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
  pub total_memory: u64,
  pub name: Option<String>,
  pub hostname: Option<String>,
  pub kernel_version: Option<String>,
  pub cpus: Vec<CpuInfo>,
  pub mnts: Vec<MntInfo>,
  pub nics: Vec<NicInfo>,
  pub blks: Vec<BlkInfo>,
  pub uts: Option<UtsInfo>,
}

#[cfg(target_os = "linux")]
mod linux {
  use super::*;
  use std::{
    fs::read_dir,
    io::{self, Read},
  };
  use sysinfo::{CpuRefreshKind, Disks, Networks, System};

  fn get_total_memory() -> u64 {
    let mut sysinfo_ = System::new_all();
    sysinfo_.refresh_memory();
    sysinfo_.total_memory()
  }

  fn get_cpu_info() -> Vec<CpuInfo> {
    let mut sysinfo_ = System::new_all();
    sysinfo_.refresh_cpu_list(CpuRefreshKind::nothing());
    sysinfo_.cpus().iter().fold(Vec::with_capacity(4), |mut list, item| {
      for added in &mut list {
        if added.vendor_id == item.vendor_id() && added.brand == item.brand() {
          added.names.push(item.name().to_string());
          return list;
        }
      }
      list.push(CpuInfo {
        names: vec![item.name().to_string()],
        vendor_id: item.vendor_id().to_string(),
        brand: item.brand().to_string(),
      });
      list
    })
  }

  fn get_mnt_info() -> Vec<MntInfo> {
    Disks::new_with_refreshed_list()
      .iter()
      .map(|disk| MntInfo {
        kind: disk.kind().to_string(),
        device_name: disk.name().to_str().unwrap_or_default().to_string(),
        file_system: disk.file_system().to_str().unwrap_or_default().to_string(),
        mount_point: disk.mount_point().to_str().unwrap_or_default().to_string(),
        total_space: disk.total_space(),
        is_removable: disk.is_removable(),
        is_read_only: disk.is_read_only(),
      })
      .collect()
  }

  fn get_nic_info() -> Vec<NicInfo> {
    Networks::new_with_refreshed_list()
      .values()
      .map(|network| NicInfo {
        mac_address: network.mac_address().to_string(),
        mtu: network.mtu(),
        ip: network
          .ip_networks()
          .iter()
          .map(|ip| IpInfo {
            addr: ip.addr.to_string(),
            version: if ip.addr.is_ipv4() { 4 } else { 6 },
            prefix: ip.prefix,
          })
          .collect(),
      })
      .collect()
  }

  fn get_blk_info() -> Vec<BlkInfo> {
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

  fn get_uts_info() -> Option<UtsInfo> {
    nix::sys::utsname::uname()
      .map(|uts| UtsInfo {
        sysname: uts.sysname().to_string_lossy().to_string(),
        nodename: uts.nodename().to_string_lossy().to_string(),
        release: uts.release().to_string_lossy().to_string(),
        version: uts.version().to_string_lossy().to_string(),
        machine: uts.machine().to_string_lossy().to_string(),
        domainname: uts.domainname().to_string_lossy().to_string(),
      })
      .ok()
  }

  pub fn collect_info() -> SystemInfo {
    SystemInfo {
      total_memory: get_total_memory(),
      name: System::name(),
      hostname: System::host_name(),
      kernel_version: System::kernel_version(),
      cpus: get_cpu_info(),
      mnts: get_mnt_info(),
      nics: get_nic_info(),
      blks: get_blk_info(),
      uts: get_uts_info(),
    }
  }
}

#[cfg(target_os = "linux")]
pub use self::linux::collect_info;
