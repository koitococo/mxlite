use anyhow::Result;
use common::utils::xxh3_for_file;
use log::warn;

use crate::net::Context;
use crate::utils::{download_file, upload_file};
use common::messages::{
    AgentResponsePayload, FileOperation, FileOperationResponse, FileTransferRequest,
};

use super::TaskHandler;

pub(super) struct FileTaskParams {
    url: String,
    path: String,
}

impl FileTaskParams {
    async fn handle_download(self, ctx: Context) -> Result<()> {
        match download_file(&self.url, &self.path).await {
            Ok(hash) => {
                ctx.respond2(
                    true,
                    AgentResponsePayload::FileOperationResponse(FileOperationResponse {
                        success: true,
                        hash: Some(hash),
                    }),
                )
                .await;
            }
            Err(err) => {
                warn!(
                    "Failed to download file from '{}' to '{}': {}",
                    self.url, self.path, err
                );
                ctx.respond2(
                    false,
                    AgentResponsePayload::FileOperationResponse(FileOperationResponse {
                        success: false,
                        hash: None,
                    }),
                )
                .await;
            }
        }
        Ok(())
    }

    async fn handle_upload(self, ctx: Context) -> Result<()> {
        match upload_file(&self.url, &self.path).await {
            Ok(_) => {
                ctx.respond2(
                    true,
                    AgentResponsePayload::FileOperationResponse(FileOperationResponse {
                        success: true,
                        hash: Some(xxh3_for_file(&self.path).await?),
                    }),
                )
                .await;
            }
            Err(err) => {
                warn!(
                    "Failed to upload file from '{}' to '{}': {}",
                    self.path, self.url, err
                );
                ctx.respond2(
                    false,
                    AgentResponsePayload::FileOperationResponse(FileOperationResponse {
                        success: false,
                        hash: None,
                    }),
                )
                .await;
            }
        }

        Ok(())
    }
}

pub(super) enum FileTask {
    Download(FileTaskParams),
    Upload(FileTaskParams),
}

impl TaskHandler for FileTask {
    async fn handle(self, ctx: Context) -> Result<()> {
        match self {
            FileTask::Download(task) => task.handle_download(ctx).await,
            FileTask::Upload(task) => task.handle_upload(ctx).await,
        }
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
        }
    }
}
