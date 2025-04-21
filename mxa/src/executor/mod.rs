mod cmd_task;
mod file_task;

use anyhow::Result;
use cmd_task::ExecuteTask;
use file_task::FileTask;

use crate::net::Context;
use common::protocol::controller::{AgentResponsePayload, ControllerRequest, ControllerRequestPayload};

trait TaskHandler {
  fn handle(self) -> impl Future<Output = Result<(bool, AgentResponsePayload)>>;
}

enum Task {
  File(FileTask),
  Cmd(ExecuteTask),
}

impl From<&ControllerRequest> for Task {
  fn from(msg: &ControllerRequest) -> Self {
    match &msg.payload {
      ControllerRequestPayload::FileTransferRequest(req) => Task::File(FileTask::from(req)),
      ControllerRequestPayload::CommandExecutionRequest(req) => Task::Cmd(ExecuteTask::from(req)),
    }
  }
}

impl TaskHandler for Task {
  async fn handle(self) -> Result<(bool, AgentResponsePayload)> {
    match self {
      Task::File(task) => task.handle().await,
      Task::Cmd(task) => task.handle().await,
    }
  }
}

pub(crate) async fn handle_event(ctx: Context) -> Result<()> {
  let result = Task::from(&ctx.request).handle().await?;
  ctx.respond(result.0, result.1).await;
  Ok(())
}
