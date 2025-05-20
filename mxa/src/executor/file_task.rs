use std::{fs::OpenOptions, io::Write as _};

use crate::utils::{download_file, upload_file};
use anyhow::Result;
use common::{
  hash::xxh3_for_file,
  protocol::messaging::{
    AgentResponsePayload, FileDownloadParams, FileDownloadResult, FileOperationResponse, FileReadParams,
    FileReadResult, FileTransferRequest, FileUploadParams, FileUploadResult, FileWriteParams, FileWriteResult,
  },
};
use log::warn;

use super::TaskHandler;

trait FileOperationHandler {
  async fn handle(self) -> FileOperationResponse;
}

impl FileOperationHandler for FileDownloadParams {
  async fn handle(self) -> FileOperationResponse {
    match download_file(&self.src_url, &self.dest_path).await {
      Ok(hash) => FileDownloadResult {
        ok: true,
        hash: Some(hash),
      }
      .into(),
      Err(err) => {
        warn!(
          "Failed to download file from '{}' to '{}': {}",
          self.src_url, self.dest_path, err
        );
        FileDownloadResult { ok: false, hash: None }.into()
      }
    }
  }
}

impl FileOperationHandler for FileUploadParams {
  async fn handle(self) -> FileOperationResponse {
    match upload_file(&self.src_path, &self.dest_url).await {
      Ok(_) => FileUploadResult {
        ok: true,
        hash: xxh3_for_file(&self.src_path)
          .await
          .inspect_err(|err| {
            warn!("Failed to calculate hash for file '{}': {}", self.src_path, err);
          })
          .ok(),
      }
      .into(),
      Err(err) => {
        warn!(
          "Failed to upload file from '{}' to '{}': {}",
          self.src_path, self.dest_url, err
        );
        FileUploadResult { ok: false, hash: None }.into()
      }
    }
  }
}

impl FileOperationHandler for FileReadParams {
  async fn handle(self) -> FileOperationResponse {
    match std::fs::read(&self.src_path) {
      Ok(content) => {
        if let Some(size_limit) = self.size_limit &&
          content.len() > size_limit as usize
        {
          warn!("File '{}' exceeds size limit of {} bytes", self.src_path, size_limit);
          return FileReadResult {
            ok: false,
            size: content.len() as u64,
            content: None,
          }
          .into();
        }
        let content_str = String::from_utf8_lossy(&content).to_string();
        FileReadResult {
          ok: true,
          size: content.len() as u64,
          content: Some(content_str),
        }
        .into()
      }
      Err(err) => {
        warn!("Failed to read file '{}': {}", self.src_path, err);
        FileReadResult {
          ok: false,
          size: 0,
          content: None,
        }
        .into()
      }
    }
  }
}

impl FileOperationHandler for FileWriteParams {
  async fn handle(self) -> FileOperationResponse {
    let f = OpenOptions::new().write(true).create(true).truncate(true).open(&self.dest_path);
    match f {
      Ok(mut file) => {
        if let Err(err) = file.write_all(self.content.as_bytes()) {
          warn!("Failed to write to file '{}': {}", self.dest_path, err);
          return FileWriteResult { ok: false }.into();
        }
        FileWriteResult { ok: true }.into()
      }
      Err(err) => {
        warn!("Failed to open file '{}': {}", self.dest_path, err);
        FileWriteResult { ok: false }.into()
      }
    }
  }
}

impl FileOperationHandler for FileTransferRequest {
  async fn handle(self) -> FileOperationResponse {
    match self {
      FileTransferRequest::Download(params) => params.handle().await,
      FileTransferRequest::Upload(params) => params.handle().await,
      FileTransferRequest::Read(params) => params.handle().await,
      FileTransferRequest::Write(params) => params.handle().await,
    }
  }
}

impl TaskHandler for FileTransferRequest {
  async fn handle(self) -> Result<AgentResponsePayload> {
    let response = FileOperationHandler::handle(self).await;
    Ok(AgentResponsePayload::FileOperationResponse(response))
  }
}
