mod cmd_task;
mod file_task;
mod script_task;

use anyhow::Result;
use cmd_task::ExecutionTask;
use file_task::FileTask;
use log::warn;
use script_task::ScriptTask;

use crate::net::Context;
use common::protocol::messaging::{AgentResponsePayload, ControllerRequest, ControllerRequestPayload, ErrorResponse};

trait TaskHandler {
  fn handle(self) -> impl Future<Output = Result<AgentResponsePayload>>;
}

enum Task {
  File(FileTask),
  Cmd(ExecutionTask),
  Script(ScriptTask),
}

impl From<&ControllerRequest> for Task {
  fn from(msg: &ControllerRequest) -> Self {
    match &msg.payload {
      ControllerRequestPayload::FileTransferRequest(req) => Task::File(FileTask::from(req)),
      ControllerRequestPayload::CommandExecutionRequest(req) => Task::Cmd(ExecutionTask::from(req)),
      ControllerRequestPayload::ScriptEvalRequest(req) => Task::Script(ScriptTask::from(req)),
    }
  }
}

impl TaskHandler for Task {
  async fn handle(self) -> Result<AgentResponsePayload> {
    match self {
      Task::File(task) => task.handle().await,
      Task::Cmd(task) => task.handle().await,
      Task::Script(task) => task.handle().await,
    }
  }
}

pub(crate) async fn handle_event(ctx: Context) {
  let task_result = Task::from(&ctx.request).handle().await;
  let responding_result = match task_result {
    Ok(payload) => ctx.respond(true, payload).await,
    Err(err) => {
      warn!("Failed to handle request: {err}");
      ctx
        .respond(
          false,
          ErrorResponse {
            code: "ERR_GENERIC".to_string(),
            message: err.to_string(),
          }
          .into(),
        )
        .await
    }
  };
  if responding_result.is_err() {
    warn!("Failed to respond to request: {}", ctx.request.id);
  }
}
