use std::{fs::OpenOptions, io::Write as _};

use common::util_func::{download_file, upload_file};
use anyhow::Result;
use common::{
  hash::xxh3_for_file,
  protocol::messaging::{
    ErrorResponse, FileDownloadParams, FileDownloadResult, FileOperationResponse, FileReadParams,
    FileReadResult, FileTransferRequest, FileUploadParams, FileUploadResult, FileWriteParams, FileWriteResult,
  },
};
use log::warn;

use super::RequestHandler;

impl RequestHandler<FileDownloadResult> for FileDownloadParams {
  async fn handle(&self) -> Result<FileDownloadResult, ErrorResponse> {
    match download_file(&self.src_url, &self.dest_path).await {
      Ok(hash) => Ok(FileDownloadResult {
        ok: true,
        hash: Some(hash),
      }),
      Err(err) => {
        warn!(
          "Failed to download file from '{}' to '{}': {}",
          self.src_url, self.dest_path, err
        );
        Ok(FileDownloadResult { ok: false, hash: None })
      }
    }
  }
}

impl RequestHandler<FileUploadResult> for FileUploadParams {
  async fn handle(&self) -> Result<FileUploadResult, ErrorResponse> {
    match upload_file(&self.src_path, &self.dest_url).await {
      Ok(_) => Ok(FileUploadResult {
        ok: true,
        hash: xxh3_for_file(&self.src_path)
          .await
          .inspect_err(|err| {
            warn!("Failed to calculate hash for file '{}': {}", self.src_path, err);
          })
          .ok(),
      }),
      Err(err) => {
        warn!(
          "Failed to upload file from '{}' to '{}': {}",
          self.src_path, self.dest_url, err
        );
        Ok(FileUploadResult { ok: false, hash: None })
      }
    }
  }
}

impl RequestHandler<FileReadResult> for FileReadParams {
  async fn handle(&self) -> Result<FileReadResult, ErrorResponse> {
    match std::fs::read(&self.src_path) {
      Ok(content) => {
        if let Some(size_limit) = self.size_limit &&
          content.len() > size_limit as usize
        {
          warn!("File '{}' exceeds size limit of {} bytes", self.src_path, size_limit);
          return Ok(FileReadResult {
            ok: false,
            size: content.len() as u64,
            content: None,
          });
        }
        let content_str = String::from_utf8_lossy(&content).to_string();
        Ok(FileReadResult {
          ok: true,
          size: content.len() as u64,
          content: Some(content_str),
        })
      }
      Err(err) => {
        warn!("Failed to read file '{}': {}", self.src_path, err);
        Ok(FileReadResult {
          ok: false,
          size: 0,
          content: None,
        })
      }
    }
  }
}

impl RequestHandler<FileWriteResult> for FileWriteParams {
  async fn handle(&self) -> Result<FileWriteResult, ErrorResponse> {
    let f = OpenOptions::new().write(true).create(true).truncate(true).open(&self.dest_path);
    match f {
      Ok(mut file) => {
        if let Err(err) = file.write_all(self.content.as_bytes()) {
          warn!("Failed to write to file '{}': {}", self.dest_path, err);
          return Ok(FileWriteResult { ok: false });
        }
        Ok(FileWriteResult { ok: true })
      }
      Err(err) => {
        warn!("Failed to open file '{}': {}", self.dest_path, err);
        Ok(FileWriteResult { ok: false })
      }
    }
  }
}

impl RequestHandler<FileOperationResponse> for FileTransferRequest {
  async fn handle(&self) -> Result<FileOperationResponse, ErrorResponse> {
    let r = match self {
      FileTransferRequest::Download(params) => params.handle().await?.into(),
      FileTransferRequest::Upload(params) => params.handle().await?.into(),
      FileTransferRequest::Read(params) => params.handle().await?.into(),
      FileTransferRequest::Write(params) => params.handle().await?.into(),
    };
    Ok(r)
  }
}
