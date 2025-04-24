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