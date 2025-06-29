use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandExecutionRequest {
  pub command: String,
  pub args: Option<Vec<String>>,
  pub use_script_file: Option<bool>,
  pub use_shell: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptEvalRequest {
  pub script: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileUploadParams {
  pub src_path: String,
  pub dest_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileDownloadParams {
  pub src_url: String,
  pub dest_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileReadParams {
  pub src_path: String,
  pub size_limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileWriteParams {
  pub content: String,
  pub dest_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "operation")]
pub enum FileTransferRequest {
  Download(FileDownloadParams),
  Upload(FileUploadParams),
  Read(FileReadParams),
  Write(FileWriteParams),
}

impl From<FileDownloadParams> for FileTransferRequest {
  fn from(value: FileDownloadParams) -> Self { FileTransferRequest::Download(value) }
}
impl From<FileUploadParams> for FileTransferRequest {
  fn from(value: FileUploadParams) -> Self { FileTransferRequest::Upload(value) }
}
impl From<FileReadParams> for FileTransferRequest {
  fn from(value: FileReadParams) -> Self { FileTransferRequest::Read(value) }
}
impl From<FileWriteParams> for FileTransferRequest {
  fn from(value: FileWriteParams) -> Self { FileTransferRequest::Write(value) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ControllerRequestPayload {
  CommandExecutionRequest(CommandExecutionRequest),
  ScriptEvalRequest(ScriptEvalRequest),
  FileTransferRequest(FileTransferRequest),
}

impl From<CommandExecutionRequest> for ControllerRequestPayload {
  fn from(value: CommandExecutionRequest) -> Self { ControllerRequestPayload::CommandExecutionRequest(value) }
}
impl From<ScriptEvalRequest> for ControllerRequestPayload {
  fn from(value: ScriptEvalRequest) -> Self { ControllerRequestPayload::ScriptEvalRequest(value) }
}
impl From<FileTransferRequest> for ControllerRequestPayload {
  fn from(value: FileTransferRequest) -> Self { ControllerRequestPayload::FileTransferRequest(value) }
}
impl From<FileUploadParams> for ControllerRequestPayload {
  fn from(value: FileUploadParams) -> Self { ControllerRequestPayload::FileTransferRequest(value.into()) }
}
impl From<FileDownloadParams> for ControllerRequestPayload {
  fn from(value: FileDownloadParams) -> Self { ControllerRequestPayload::FileTransferRequest(value.into()) }
}
impl From<FileReadParams> for ControllerRequestPayload {
  fn from(value: FileReadParams) -> Self { ControllerRequestPayload::FileTransferRequest(value.into()) }
}
impl From<FileWriteParams> for ControllerRequestPayload {
  fn from(value: FileWriteParams) -> Self { ControllerRequestPayload::FileTransferRequest(value.into()) }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControllerRequest {
  pub version: u32,
  pub id: u32,
  pub payload: ControllerRequestPayload,
}

#[test]
fn test_controller_request_serialization() {
  let request = ControllerRequest {
    version: 1,
    id: 1,
    payload: ControllerRequestPayload::FileTransferRequest(FileTransferRequest::Download(FileDownloadParams {
      src_url: "http://example.com/file.txt".to_string(),
      dest_path: "/tmp/file.txt".to_string(),
    })),
  };
  let serialized = serde_json::to_string(&request).unwrap();
  println!("Serialized: {}", serialized);
  let deserialized: ControllerRequest = serde_json::from_str(&serialized).unwrap();
  println!("Deserialized: {:?}", deserialized);
}
