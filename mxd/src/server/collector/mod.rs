use common::protocol::messaging::{AgentResponse, Message};
use log::{debug, info};
use std::sync::Arc;

use crate::states::host_session::HostSession;

pub(super) async fn handle_msg(msg: Message, session: Arc<HostSession>) {
  debug!("Received message: {msg:?}");
  if let Message::AgentResponse(response) = msg {
    handle_resp(response, session.clone()).await;
  }
}

async fn handle_resp(response: AgentResponse, session: Arc<HostSession>) {
  info!("Task Completed: {} {}", session.host_id, response.id);
  session.set_task_finished(response.id, response);
}
