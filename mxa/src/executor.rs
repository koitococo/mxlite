use anyhow::Result;
use common::utils::xxh3_for_file;
use log::{trace, warn};

use crate::net::Context;
use crate::utils::{download_file, execute_shell_with_output, upload_file};
use common::messages::{
    AgentResponsePayload, CommandExecutionResponse, ControllerRequest, ControllerRequestPayload,
    FileOperation, FileOperationResponse,
};

struct FileDownloadUploadTask {
    url: String,
    path: String,
}

impl FileDownloadUploadTask {
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

struct ExecuteTask {
    cmd: String,
}

impl ExecuteTask {
    async fn handle(self, ctx: Context) -> Result<()> {
        match execute_shell_with_output(&self.cmd).await {
            Ok((code, stdout, stderr)) => {
                trace!(
                    "Command '{}' executed with code {}: {} {}",
                    self.cmd, code, stdout, stderr
                );
                ctx.respond2(
                    true,
                    AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse {
                        code,
                        stdout,
                        stderr,
                    }),
                )
                .await;
            }
            Err(err) => {
                warn!("Failed to execute command {}: {}", self.cmd, err);
                ctx.respond2(
                    false,
                    AgentResponsePayload::CommandExecutionResponse(CommandExecutionResponse {
                        code: -1,
                        stdout: "".to_string(),
                        stderr: err.to_string(),
                    }),
                )
                .await;
            }
        }
        Ok(())
    }
}

enum Task {
    Download(FileDownloadUploadTask),
    Upload(FileDownloadUploadTask),
    Execute(ExecuteTask),
}

impl Task {
    async fn handle(self, ctx: Context) -> Result<()> {
        match self {
            Task::Download(task) => task.handle_download(ctx).await,
            Task::Upload(task) => task.handle_upload(ctx).await,
            Task::Execute(task) => task.handle(ctx).await,
        }
    }
}

impl TryFrom<&ControllerRequest> for Task {
    type Error = ();

    fn try_from(msg: &ControllerRequest) -> Result<Self, Self::Error> {
        match &msg.payload {
            ControllerRequestPayload::FileTransferRequest(req) => match req.operation {
                FileOperation::Download => Ok(Task::Download(FileDownloadUploadTask {
                    url: req.url.clone(),
                    path: req.path.clone(),
                })),
                FileOperation::Upload => Ok(Task::Upload(FileDownloadUploadTask {
                    url: req.url.clone(),
                    path: req.path.clone(),
                })),
            },
            ControllerRequestPayload::CommandExecutionRequest(req) => {
                Ok(Task::Execute(ExecuteTask {
                    cmd: req.command.clone(),
                }))
            }
        }
    }
}

pub(crate) async fn handle_event(ctx: Context) -> Result<()> {
    if let Ok(task) = Task::try_from(&ctx.request) {
        task.handle(ctx).await
    } else {
        warn!("Received an invalid task: {:?}", ctx.request);
        Err(anyhow::anyhow!("Invalid task"))
    }
}
