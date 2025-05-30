mod cmd_task;
mod file_task;
mod script_task;

use log::warn;

use crate::net::Context;
use common::protocol::messaging::{AgentResponsePayload, ControllerRequest, ControllerRequestPayload, ErrorResponse};

trait RequestHandler<T> {
  async fn handle(&self) -> Result<T, ErrorResponse>;
}

impl RequestHandler<AgentResponsePayload> for ControllerRequest {
  async fn handle(&self) -> Result<AgentResponsePayload, ErrorResponse> {
    let r = match &self.payload {
      ControllerRequestPayload::CommandExecutionRequest(req) => req.handle().await?.into(),
      ControllerRequestPayload::ScriptEvalRequest(req) => req.handle().await?.into(),
      ControllerRequestPayload::FileTransferRequest(req) => req.handle().await?.into(),
    };
    Ok(r)
  }
}

pub(crate) async fn handle_event(ctx: Context) {
  let task_result = ctx.request.handle().await;
  let responding_result = match task_result {
    Ok(payload) => ctx.respond(true, payload).await,
    Err(err) => {
      warn!("Failed to handle request: {}", err.message);
      ctx.respond(false, err.into()).await
    }
  };
  if responding_result.is_err() {
    warn!("Failed to respond to request: {}", ctx.request.id);
  }
}
