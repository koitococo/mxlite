use std::str::FromStr;

use base64::{DecodeError, Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::system_info::SystemInfo;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectHandshake {
  pub version: u32,
  pub host_id: String,
  pub session_id: String,
  pub envs: Vec<String>,
  pub controller_url: String,
  pub system_info: SystemInfo,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ConnectHandshake {
  fn to_string(&self) -> String { general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_string(&self).unwrap().as_bytes()).to_string() }
}

#[derive(Debug, Error)]
pub enum HandshakeError {
  #[error("Base64 decoding error: {0}")]
  Base64Error(#[from] DecodeError),
  #[error("JSON parsing error: {0}")]
  JsonError(#[from] serde_json::Error),
}

impl FromStr for ConnectHandshake {
  type Err = HandshakeError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(s)?;
    Ok(serde_json::from_slice(&decoded)?)
  }
}

pub const CONNECT_HANDSHAKE_HEADER_KEY: &str = "X-MxLite-ConnectHandshake";
