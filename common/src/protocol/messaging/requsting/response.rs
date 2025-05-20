use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionResponse {
  pub code: i32,
  pub stdout: String,
  pub stderr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptEvalResponse {
  pub ok: bool,
  pub result: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileDownloadResult {
  pub ok: bool,
  pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileUploadResult {
  pub ok: bool,
  pub hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileReadResult {
  pub ok: bool,
  pub size: u64,
  pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileWriteResult {
  pub ok: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "operation")]
pub enum FileOperationResponse {
  Download(FileDownloadResult),
  Upload(FileUploadResult),
  Read(FileReadResult),
  Write(FileWriteResult),
}

impl From<FileDownloadResult> for FileOperationResponse {
  fn from(value: FileDownloadResult) -> Self { FileOperationResponse::Download(value) }
}
impl From<FileUploadResult> for FileOperationResponse {
  fn from(value: FileUploadResult) -> Self { FileOperationResponse::Upload(value) }
}
impl From<FileReadResult> for FileOperationResponse {
  fn from(value: FileReadResult) -> Self { FileOperationResponse::Read(value) }
}
impl From<FileWriteResult> for FileOperationResponse {
  fn from(value: FileWriteResult) -> Self { FileOperationResponse::Write(value) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorResponse {
  pub code: String,
  pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum AgentResponsePayload {
  CommandExecutionResponse(CommandExecutionResponse),
  ScriptEvalResponse(ScriptEvalResponse),
  FileOperationResponse(FileOperationResponse),
  Error(ErrorResponse),
}

impl From<CommandExecutionResponse> for AgentResponsePayload {
  fn from(value: CommandExecutionResponse) -> Self { AgentResponsePayload::CommandExecutionResponse(value) }
}
impl From<ScriptEvalResponse> for AgentResponsePayload {
  fn from(value: ScriptEvalResponse) -> Self { AgentResponsePayload::ScriptEvalResponse(value) }
}
impl From<FileOperationResponse> for AgentResponsePayload {
  fn from(value: FileOperationResponse) -> Self { AgentResponsePayload::FileOperationResponse(value) }
}
impl From<ErrorResponse> for AgentResponsePayload {
  fn from(value: ErrorResponse) -> Self { AgentResponsePayload::Error(value) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentResponse {
  pub id: u64,
  pub ok: bool,
  pub payload: AgentResponsePayload,
}

#[test]
fn test_agent_response_serialization() {
  let response = AgentResponse {
    id: 1,
    ok: true,
    payload: AgentResponsePayload::FileOperationResponse(FileOperationResponse::Download(FileDownloadResult {
      ok: true,
      hash: Some("dummy_hash".to_string()),
    })),
  };
  let serialized = serde_json::to_string(&response).unwrap();
  println!("Serialized: {}", serialized);
  let deserialized: AgentResponse = serde_json::from_str(&serialized).unwrap();
  println!("Deserialized: {:?}", deserialized);
}
