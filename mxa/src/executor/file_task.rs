use std::{fs::OpenOptions, io::Write as _};

use crate::utils::{download_file, upload_file};
use anyhow::Result;
use common::{
  hash::xxh3_for_file,
  protocol::messaging::{AgentResponsePayload, FileOperation, FileOperationResponse, FileTransferRequest},
};
use log::warn;

use super::TaskHandler;

pub(super) struct FileTaskParams {
  url: String,
  path: String,
}

impl FileTaskParams {
  async fn handle_download(self) -> FileOperationResponse {
    match download_file(&self.url, &self.path).await {
      Ok(hash) => FileOperationResponse {
        success: true,
        hash: Some(hash),
      },
      Err(err) => {
        warn!(
          "Failed to download file from '{}' to '{}': {}",
          self.url, self.path, err
        );
        FileOperationResponse {
          success: false,
          hash: None,
        }
      }
    }
  }

  async fn handle_upload(self) -> FileOperationResponse {
    match upload_file(&self.url, &self.path).await {
      Ok(_) => FileOperationResponse {
        success: true,
        hash: xxh3_for_file(&self.path)
          .await
          .inspect_err(|err| {
            warn!("Failed to calculate hash for file '{}': {}", self.path, err);
          })
          .ok(),
      },
      Err(err) => {
        warn!("Failed to upload file from '{}' to '{}': {}", self.path, self.url, err);
        FileOperationResponse {
          success: false,
          hash: None,
        }
      }
    }
  }

  async fn handle_read(self) -> FileOperationResponse { todo!() }

  async fn handle_write(self) -> FileOperationResponse {
    let f = OpenOptions::new().write(true).create(true).truncate(true).open(&self.path);
    match f {
      Ok(mut file) => {
        if let Err(err) = file.write_all(self.url.as_bytes()) {
          warn!("Failed to write to file '{}': {}", self.path, err);
          return FileOperationResponse {
            success: false,
            hash: None,
          };
        }
        FileOperationResponse {
          success: true,
          hash: None,
        }
      }
      Err(err) => {
        warn!("Failed to open file '{}': {}", self.path, err);
        FileOperationResponse {
          success: false,
          hash: None,
        }
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
  async fn handle(self) -> Result<AgentResponsePayload> {
    let result = match self {
      FileTask::Download(task) => task.handle_download().await,
      FileTask::Upload(task) => task.handle_upload().await,
      FileTask::Read(task) => task.handle_read().await,
      FileTask::Write(task) => task.handle_write().await,
    };
    Ok(result.into())
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
