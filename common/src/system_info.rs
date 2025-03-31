use serde::{Deserialize, Serialize};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub names: Vec<String>,
    pub vendor_id: String,
    pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
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
pub struct SystemInfo {
    pub total_memory: u64,
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub cpus: Vec<CpuInfo>,
    pub disks: Vec<DiskInfo>,
    pub nics: Vec<NicInfo>,
}

impl SystemInfo {
    pub fn collect_info() -> Self {
        let mut sysinfo_ = System::new_all();
        sysinfo_.refresh_all();

        SystemInfo {
            total_memory: sysinfo_.total_memory(),
            name: System::name(),
            kernel_version: System::kernel_version(),
            cpus: sysinfo_.cpus().iter().fold(Vec::with_capacity(4), |mut list, item| {
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
            }),
            disks: Disks::new_with_refreshed_list()
                .iter()
                .map(|disk| DiskInfo {
                    kind: disk.kind().to_string(),
                    device_name: disk.name().to_str().unwrap_or_default().to_string(),
                    file_system: disk.file_system().to_str().unwrap_or_default().to_string(),
                    mount_point: disk.mount_point().to_str().unwrap_or_default().to_string(),
                    total_space: disk.total_space(),
                    is_removable: disk.is_removable(),
                    is_read_only: disk.is_read_only(),
                })
                .collect(),
            nics: Networks::new_with_refreshed_list()
                .iter()
                .map(|(_, network)| NicInfo {
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
                .collect(),
        }
    }
}
