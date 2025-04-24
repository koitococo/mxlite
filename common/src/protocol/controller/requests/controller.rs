use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionRequest {
  pub command: String,
  pub use_script_file: bool,
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