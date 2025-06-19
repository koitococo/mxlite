#![cfg(unix)]

mod linux;
mod macos;

use super::{BlkInfo, CpuInfo, IpInfo, MntInfo, NicInfo, SystemInfo, UtsInfo};

fn get_cpu_info() -> Option<CpuInfo> {
  // TODO: refactor this to cleanup usage of `sysinfo` crate
  let mut sysinfo_ = sysinfo::System::new_all();
  sysinfo_.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
  sysinfo_.cpus().first().map(|cpu| CpuInfo {
    brand: cpu.brand().to_string(),
    name: cpu.name().to_string(),
    vendor_id: cpu.vendor_id().to_string(),
  })
}

fn get_mnt_info() -> Vec<MntInfo> {
  // TODO: refactor this to cleanup usage of `sysinfo` crate
  sysinfo::Disks::new_with_refreshed_list()
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
  // TODO: refactor this to cleanup usage of `sysinfo` crate
  sysinfo::Networks::new_with_refreshed_list()
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
      carrier: None,
      link_speed: None,
      link_state: None,
    })
    .collect()
}

fn get_blk_info() -> Vec<BlkInfo> {
  #[cfg(target_os = "linux")]
  return linux::get_blk_info();

  #[cfg(target_os = "macos")]
  return macos::get_blk_info();

  #[cfg(not(any(target_os = "linux", target_os = "macos")))]
  return vec![]; // No block info available for other Unix systems
}

fn get_uts_info() -> Option<UtsInfo> {
  nix::sys::utsname::uname()
    .map(|uts| UtsInfo {
      sysname: uts.sysname().to_string_lossy().to_string(),
      nodename: uts.nodename().to_string_lossy().to_string(),
      release: uts.release().to_string_lossy().to_string(),
      version: uts.version().to_string_lossy().to_string(),
      machine: uts.machine().to_string_lossy().to_string(),
      domainname: {
        #[cfg(target_os = "linux")]
        {
          Some(uts.domainname().to_string_lossy().to_string())
        }

        #[cfg(not(target_os = "linux"))]
        {
          None
        }
      },
    })
    .ok()
}

pub fn collect_info() -> SystemInfo {
  let uts = get_uts_info();
  SystemInfo {
    total_memory: {
      // TODO: refactor this to cleanup usage of `sysinfo` crate
      let mut sysinfo_ = sysinfo::System::new_all();
      sysinfo_.refresh_memory();
      sysinfo_.total_memory()
    },
    name: uts.as_ref().map(|u| u.sysname.clone()),
    hostname: uts.as_ref().map(|u| u.nodename.clone()),
    kernel_version: uts.as_ref().map(|u| u.release.clone()),
    cpus: get_cpu_info(),
    mnts: get_mnt_info(),
    nics: get_nic_info(),
    blks: get_blk_info(),
    uts,
  }
}
