use crate::{
  protocol::messaging::{AgentResponse, Message},
  utils::states::States,
};
use log::{debug, info};
use std::sync::Arc;

use crate::daemon::states::host_session::HostSession;

pub(super) async fn handle_msg(msg: Message, session: Arc<HostSession>) {
  debug!("Received message: {msg:?}");
  if let Message::AgentResponse(response) = msg {
    handle_resp(response, session.clone()).await;
  }
}

async fn handle_resp(response: AgentResponse, session: Arc<HostSession>) {
  info!("Task Completed: {} {}", session.host_id, response.id);
  session.tasks.insert(response.id, Some(response));
}
