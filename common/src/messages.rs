use std::str::FromStr;

use base64::{DecodeError, Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::system_info::SystemInfo;

pub const PROTOCOL_VERSION: u32 = 3;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionRequest {
    pub command: String,
    pub use_script_file: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionResponse {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum FileOperation {
    Download,
    Upload,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTransferRequest {
    pub url: String,
    pub path: String,
    pub operation: FileOperation,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileOperationResponse {
    pub success: bool,
    pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ControllerRequestPayload {
    // None,
    CommandExecutionRequest(CommandExecutionRequest),
    FileTransferRequest(FileTransferRequest),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerRequest {
    pub version: u32,
    pub id: u64,
    pub payload: ControllerRequestPayload,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum AgentResponsePayload {
    None,
    CommandExecutionResponse(CommandExecutionResponse),
    FileOperationResponse(FileOperationResponse),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentResponse {
    pub id: u64,
    pub ok: bool,
    pub payload: AgentResponsePayload,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerEvent {
    pub id: u64,
    pub event: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentEvent {
    pub id: u64,
    pub event: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerMessage {
    pub request: ControllerRequest,
    pub events: Option<Vec<ControllerEvent>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentMessage {
    pub response: Option<AgentResponse>,
    pub events: Option<Vec<AgentEvent>>,
}

impl FromStr for ControllerMessage {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ControllerMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl FromStr for AgentMessage {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for AgentMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectHandshake {
    pub version: u32,
    pub host_id: String,
    pub controller_url: String,
    pub system_info: SystemInfo,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ConnectHandshake {
    fn to_string(&self) -> String {
        general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&self).unwrap().as_bytes())
            .to_string()
    }
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
