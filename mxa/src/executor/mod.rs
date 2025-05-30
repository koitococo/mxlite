mod cmd_task;
mod file_task;
mod script_task;

use log::warn;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::Message;

use common::protocol::messaging::{
  AgentResponse, AgentResponsePayload, ControllerRequest, ControllerRequestPayload, ErrorResponse, Status,
};

use crate::net::MessageSend as _;

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

pub(crate) async fn handle_event(request: ControllerRequest, tx: Sender<Message>) {
  match request.handle().await {
    Ok(payload) => tx.send_msg(AgentResponse {
      id: request.id,
      status: Status::Ok,
      payload,
    }),
    Err(err) => {
      warn!("Failed to handle request: {}", err.message);
      tx.send_msg(AgentResponse {
        id: request.id,
        status: Status::Error,
        payload: err.into(),
      })
    }
  };
}
