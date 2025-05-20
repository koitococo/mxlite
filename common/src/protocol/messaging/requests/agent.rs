use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionResponse {
  pub code: i32,
  pub stdout: String,
  pub stderr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileOperationResponse {
  pub success: bool,
  pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptEvalResponse {
  pub ok: bool,
  pub result: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorResponse {
  pub code: String,
  pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum AgentResponsePayload {
  None,
  CommandExecutionResponse(CommandExecutionResponse),
  FileOperationResponse(FileOperationResponse),
  ScriptEvalResponse(ScriptEvalResponse),
  Error(ErrorResponse),
}

impl From<CommandExecutionResponse> for AgentResponsePayload {
  fn from(value: CommandExecutionResponse) -> Self {
    AgentResponsePayload::CommandExecutionResponse(value)
  }
}
impl From<FileOperationResponse> for AgentResponsePayload {
  fn from(value: FileOperationResponse) -> Self {
    AgentResponsePayload::FileOperationResponse(value)
  }
}
impl From<ScriptEvalResponse> for AgentResponsePayload {
  fn from(value: ScriptEvalResponse) -> Self {
    AgentResponsePayload::ScriptEvalResponse(value)
  }
}
impl From<ErrorResponse> for AgentResponsePayload {
  fn from(value: ErrorResponse) -> Self {
    AgentResponsePayload::Error(value)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentResponse {
  pub id: u64,
  pub ok: bool,
  pub payload: AgentResponsePayload,
}
