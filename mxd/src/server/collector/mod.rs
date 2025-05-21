use common::protocol::controller::{AgentEvent, AgentMessage, AgentResponse};
use futures_util::future::join_all;
use log::{debug, info};
use std::sync::Arc;

use crate::states::host_session::HostSession;

pub(super) async fn handle_msg(msg: AgentMessage, session: Arc<HostSession>) {
  debug!("Received message: {msg:?}");
  if let Some(response) = msg.response {
    handle_resp(response, session.clone()).await;
  }
  if let Some(events) = msg.events {
    handle_events(events, session).await;
  }
}

async fn handle_resp(response: AgentResponse, session: Arc<HostSession>) {
  info!("Task Completed: {} {}", session.host_id, response.id);
  session.set_task_finished(response.id, response);
}

async fn handle_events(events: Vec<AgentEvent>, session: Arc<HostSession>) {
  join_all(events.iter().map(async |event| {
    handle_event(event, session.clone()).await;
  }))
  .await;
}

async fn handle_event(event: &AgentEvent, session: Arc<HostSession>) {
  info!("Host {} Received event: {:?}", session.host_id, event);
}
