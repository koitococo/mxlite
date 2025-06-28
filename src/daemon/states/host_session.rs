use std::{clone::Clone, sync::Arc};

use crate::{
  daemon::server::SocketConnectInfo,
  protocol::messaging::{AgentResponse, ControllerRequest, ControllerRequestPayload, Message, PROTOCOL_VERSION},
  system_info::SystemInfo,
  utils::{
    mq::{Mq, VecMq},
    states::{AtomicStates, States as _},
  },
};
use anyhow::Result;
use log::{debug, warn};
use serde::Serialize;
use tokio::sync::{
  Mutex, Notify,
  mpsc::{self, Receiver, Sender, error::SendError},
};
use url::Url;

#[derive(Clone)]
pub enum TaskState {
  Pending,
  Finished(AgentResponse),
}

#[derive(Serialize, Debug, Clone)]
pub struct ExtraInfo {
  pub socket_info: SocketConnectInfo,
  pub controller_url: Url,
  pub system_info: SystemInfo,
  pub envs: Vec<String>,
  pub session_id: String,
}

pub struct HostSession {
  pub host_id: String,
  pub session_id: String,
  tx: Sender<Message>,
  rx: Mutex<Receiver<Message>>,
  tasks: VecMq<u64, TaskState>,
  pub extra: ExtraInfo,
  pub notify: Notify,
}

impl HostSession {
  pub fn new(host_id: String, extra: ExtraInfo) -> Self {
    let (tx, rx) = mpsc::channel(32);
    HostSession {
      host_id,
      session_id: extra.session_id.clone(),
      tx,
      rx: Mutex::new(rx),
      tasks: VecMq::new(128),
      extra,
      notify: Notify::new(),
    }
  }

  pub(crate) async fn send_req(&self, req: ControllerRequest) -> Result<(), SendError<Message>> {
    self.tx.send(Message::ControllerRequest(req)).await
  }

  pub(crate) async fn recv_req(&self) -> Option<ControllerRequest> {
    if let Some(Message::ControllerRequest(req)) = self.rx.lock().await.recv().await {
      Some(req)
    } else {
      None
    }
  }

  pub(crate) fn new_task(&self) -> u64 {
    loop {
      let id: u64 = rand::random::<u64>() >> 16;
      if self.tasks.send(id, TaskState::Pending) {
        return id;
      }
    }
  }

  pub(crate) fn set_task_finished(&self, id: u64, resp: AgentResponse) {
    if !self.tasks.send(id, TaskState::Finished(resp)) {
      warn!("Failed to set task state for id: {id}");
    }
  }

  pub(crate) fn get_task_state(&self, id: u64) -> Option<Arc<TaskState>> { self.tasks.receive(&id) }
}

pub(crate) type HostSessionStorage = AtomicStates<String, HostSession>;
pub(crate) trait HostSessionStorageExt {
  fn create_session(&self, host_id: &String, extra: ExtraInfo) -> Option<Arc<HostSession>>;
  async fn send_req(&self, id: &String, req: ControllerRequestPayload) -> Option<Result<u64, SendError<Message>>>;
  async fn get_resp(&self, id: &String, task_id: u64) -> Option<Option<TaskState>>;
  async fn list_all_tasks(&self, id: &String) -> Vec<u64>;
}

impl HostSessionStorageExt for HostSessionStorage {
  fn create_session(&self, host_id: &String, extra: ExtraInfo) -> Option<Arc<HostSession>> {
    self.try_insert_deferred_returning(host_id.clone(), || HostSession::new(host_id.to_string(), extra))
  }

  async fn send_req(&self, id: &String, req: ControllerRequestPayload) -> Option<Result<u64, SendError<Message>>> {
    if let Some(session) = self.get(id) {
      debug!("Sending request to session: {}", session.host_id);
      let task_id = session.new_task();
      if let Err(e) = session
        .send_req(ControllerRequest {
          version: PROTOCOL_VERSION,
          id: task_id,
          payload: req,
        })
        .await
      {
        Some(Err(e))
      } else {
        Some(Ok(task_id))
      }
    } else {
      None
    }
  }

  async fn get_resp(&self, id: &String, task_id: u64) -> Option<Option<TaskState>> {
    self.get(id).map(|session| session.get_task_state(task_id).map(|task| task.as_ref().clone()))
  }

  async fn list_all_tasks(&self, id: &String) -> Vec<u64> {
    if let Some(session) = self.get(id) {
      session.tasks.list()
    } else {
      vec![]
    }
  }
}
