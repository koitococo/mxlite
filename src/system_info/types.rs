use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
  pub name: String,
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
  /// true if the NIC is connected, false otherwise
  pub carrier: Option<bool>,
  /// e.g., "up", "down"
  pub link_state: Option<String>,
  /// in Mbps, e.g., 1000 for 1 Gbps
  pub link_speed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlkInfo {
  /// major:minor number, e.g., 8:0, 8:1
  pub maj_min: String,
  /// disk sequence number, e.g., 0, 1
  pub disk_seq: u64,
  /// device name, e.g., sda, sdb
  pub name: String,
  /// kernel blk name, e.g., sda, sdb
  pub kname: String,
  /// device model, e.g., WDS500G3X0C-00SJG0
  pub model: Option<String>,
  /// device size in bytes
  pub size: u64,
  /// whether the device is removable
  pub removable: bool,
  /// UUID of the device
  pub uuid: Option<String>,
  /// WWID of the device
  pub wwid: Option<String>,
  /// whether the device is read-only
  pub readonly: bool,
  /// device path, e.g., /dev/sda
  pub path: Option<String>,
  /// device path by sequence number, e.g., /dev/disk/by-seq/0
  pub path_by_seq: Option<String>,
  /// subsystem of the device, e.g., nvme, scsi
  pub subsystem: Option<String>,
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
  pub cpus: Option<CpuInfo>,
  pub mnts: Vec<MntInfo>,
  pub nics: Vec<NicInfo>,
  pub blks: Vec<BlkInfo>,
  pub uts: Option<UtsInfo>,
}

impl Default for SystemInfo {
  fn default() -> Self {
    SystemInfo {
      total_memory: u64::MAX,
      name: None,
      hostname: None,
      kernel_version: None,
      cpus: None,
      mnts: Vec::with_capacity(0),
      nics: Vec::with_capacity(0),
      blks: Vec::with_capacity(0),
      uts: None,
    }
  }
}
