use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const PROTOCOL_REV: u32 = 1;
pub const MAGIC_REQUEST: &str = "MXA-DISCOVER";
pub const MAGIC_RESPONSE: &str = "MXA-RESPONSE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    pub magic: String,
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

#[allow(clippy::to_string_trait_impl)]
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

#[allow(clippy::to_string_trait_impl)]
impl ToString for DiscoveryRequest {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub const DISCOVERY_PORT: u16 = 11451;
