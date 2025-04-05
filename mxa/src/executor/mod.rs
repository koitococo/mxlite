mod cmd_task;
mod file_task;

use anyhow::Result;
use cmd_task::ExecuteTask;
use file_task::FileTask;

use crate::net::Context;
use common::messages::{ControllerRequest, ControllerRequestPayload};

trait TaskHandler {
    async fn handle(self, ctx: Context) -> Result<()>;
}

enum Task {
    File(FileTask),
    Cmd(ExecuteTask),
}

impl From<&ControllerRequest> for Task {
    fn from(msg: &ControllerRequest) -> Self {
        match &msg.payload {
            ControllerRequestPayload::FileTransferRequest(req) => Task::File(FileTask::from(req)),
            ControllerRequestPayload::CommandExecutionRequest(req) => {
                Task::Cmd(ExecuteTask::from(req))
            }
        }
    }
}

impl TaskHandler for Task {
    async fn handle(self, ctx: Context) -> Result<()> {
        match self {
            Task::File(task) => task.handle(ctx).await,
            Task::Cmd(task) => task.handle(ctx).await,
        }
    }
}

pub(crate) async fn handle_event(ctx: Context) -> Result<()> {
    Task::from(&ctx.request).handle(ctx).await
}
