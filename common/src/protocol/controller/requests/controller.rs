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
pub struct ScriptEvalRequest {
  pub script: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ControllerRequestPayload {
  // None,
  CommandExecutionRequest(CommandExecutionRequest),
  FileTransferRequest(FileTransferRequest),
  ScriptEvalRequest(ScriptEvalRequest),
}

impl From<CommandExecutionRequest> for ControllerRequestPayload {
  fn from(value: CommandExecutionRequest) -> Self { ControllerRequestPayload::CommandExecutionRequest(value) }
}
impl From<FileTransferRequest> for ControllerRequestPayload {
  fn from(value: FileTransferRequest) -> Self { ControllerRequestPayload::FileTransferRequest(value) }
}
impl From<ScriptEvalRequest> for ControllerRequestPayload {
  fn from(value: ScriptEvalRequest) -> Self { ControllerRequestPayload::ScriptEvalRequest(value) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerRequest {
  pub version: u32,
  pub id: u64,
  pub payload: ControllerRequestPayload,
}
