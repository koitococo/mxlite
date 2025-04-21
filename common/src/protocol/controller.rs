use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
  Read,
  Write,
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

  fn from_str(s: &str) -> Result<Self, Self::Err> { serde_json::from_str(s) }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ControllerMessage {
  fn to_string(&self) -> String { serde_json::to_string(self).unwrap() }
}

impl FromStr for AgentMessage {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> { serde_json::from_str(s) }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for AgentMessage {
  fn to_string(&self) -> String { serde_json::to_string(self).unwrap() }
}

// pub const CLOSE_CODE : u16 = 1000;
// pub const CLOSE_MXA_SHUTDOWN: &str = "MXA_SHUTDOWN";
