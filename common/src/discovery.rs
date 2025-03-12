use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

pub const PROTOCOL_REV: u32 = 1;
pub const MAGIC_REQUEST: &str = "MXA-DISCOVER";
pub const MAGIC_RESPONSE: &str = "MXA-RESPONSE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    pub magic: String,
    pub addr: SocketAddr,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    pub magic: String,
    pub ws: Vec<String>,
}

impl FromStr for DiscoveryResponse {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<DiscoveryResponse>(s)
    }
}

impl ToString for DiscoveryResponse {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl FromStr for DiscoveryRequest {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<DiscoveryRequest>(s)
    }
}

impl ToString for DiscoveryRequest {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub fn get_multicast_ipaddr() -> Ipv4Addr {
    Ipv4Addr::new(224, 233, 233, 233)
}

pub fn get_multicast_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(get_multicast_ipaddr()), 11451)
}
