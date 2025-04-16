use std::{fs::OpenOptions, io::Write as _};

use anyhow::Result;
use common::utils::xxh3_for_file;
use log::warn;

use crate::utils::{download_file, upload_file};
use common::protocol::controller::{AgentResponsePayload, FileOperation, FileOperationResponse, FileTransferRequest};

use super::TaskHandler;

pub(super) struct FileTaskParams {
  url: String,
  path: String,
}

impl FileTaskParams {
  async fn handle_download(self) -> Result<FileOperationResponse> {
    match download_file(&self.url, &self.path).await {
      Ok(hash) => Ok(FileOperationResponse {
        success: true,
        hash: Some(hash),
      }),
      Err(err) => {
        warn!("Failed to download file from '{}' to '{}': {}", self.url, self.path, err);
        Ok(FileOperationResponse { success: false, hash: None })
      }
    }
  }

  async fn handle_upload(self) -> Result<FileOperationResponse> {
    match upload_file(&self.url, &self.path).await {
      Ok(_) => Ok(FileOperationResponse {
        success: true,
        hash: Some(xxh3_for_file(&self.path).await?),
      }),
      Err(err) => {
        warn!("Failed to upload file from '{}' to '{}': {}", self.path, self.url, err);
        Ok(FileOperationResponse { success: false, hash: None })
      }
    }
  }

  async fn handle_read(self) -> Result<FileOperationResponse> { todo!() }

  async fn handle_write(self) -> Result<FileOperationResponse> {
    let f = OpenOptions::new().write(true).create(true).truncate(true).open(&self.path);
    match f {
      Ok(mut file) => {
        if let Err(err) = file.write_all(self.url.as_bytes()) {
          warn!("Failed to write to file '{}': {}", self.path, err);
          return Ok(FileOperationResponse { success: false, hash: None });
        }
        Ok(FileOperationResponse { success: true, hash: None })
      }
      Err(err) => {
        warn!("Failed to open file '{}': {}", self.path, err);
        Ok(FileOperationResponse { success: false, hash: None })
      }
    }
  }
}

pub(super) enum FileTask {
  Download(FileTaskParams),
  Upload(FileTaskParams),
  Read(FileTaskParams),
  Write(FileTaskParams),
}

impl TaskHandler for FileTask {
  async fn handle(self) -> Result<(bool, AgentResponsePayload)> {
    let result = match self {
      FileTask::Download(task) => task.handle_download().await?,
      FileTask::Upload(task) => task.handle_upload().await?,
      FileTask::Read(task) => task.handle_read().await?,
      FileTask::Write(task) => task.handle_write().await?,
    };
    Ok((result.success, AgentResponsePayload::FileOperationResponse(result)))
  }
}
impl From<&FileTransferRequest> for FileTask {
  fn from(value: &FileTransferRequest) -> Self {
    let params = FileTaskParams {
      url: value.url.clone(),
      path: value.path.clone(),
    };

    match value.operation {
      FileOperation::Download => FileTask::Download(params),
      FileOperation::Upload => FileTask::Upload(params),
      FileOperation::Read => FileTask::Read(params),
      FileOperation::Write => FileTask::Write(params),
    }
  }
}
