mod cmd_task;
mod file_task;
mod script_task;

use log::warn;

use crate::protocol::messaging::{
  AgentResponse, AgentResponsePayload, ControllerRequest, ControllerRequestPayload, ErrorResponse, Status,
};

use crate::agent::net::{MessageSend as _, MessageSender};

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

pub(crate) async fn handle_event(request: ControllerRequest, tx: MessageSender) {
  match request.handle().await {
    Ok(payload) => tx.send_msg(AgentResponse {
      id: request.id,
      status: Status::Ok,
      payload,
    }),
    Err(err) => {
      warn!("Failed to handle request: {err:?}");
      tx.send_msg(AgentResponse {
        id: request.id,
        status: Status::Error,
        payload: err.into(),
      })
    }
  };
}
