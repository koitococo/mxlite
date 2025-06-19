mod types;
mod unix;

pub use self::types::{BlkInfo, CpuInfo, IpInfo, MntInfo, NicInfo, SystemInfo, UtsInfo};

#[cfg(unix)]
pub use self::unix::collect_info;

#[cfg(not(unix))]
pub fn collect_info() -> SystemInfo { SystemInfo::default() }
