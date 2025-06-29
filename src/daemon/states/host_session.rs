use std::clone::Clone;

use crate::{
  daemon::server::SocketConnectInfo,
  protocol::messaging::{AgentResponse, ControllerRequest, ControllerRequestPayload, Message, PROTOCOL_VERSION},
  system_info::SystemInfo,
  utils::states::{StateMap, States as _},
};
use anyhow::Result;
use log::debug;
use serde::Serialize;
use tokio::sync::{
  Mutex, Notify,
  mpsc::{self, Receiver, Sender, error::SendError},
};
use url::Url;

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
  pub tasks: StateMap<u32, Option<AgentResponse>>,
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
      tasks: StateMap::new(),
      extra,
      notify: Notify::new(),
    }
  }

  pub async fn send_req(&self, req: ControllerRequest) -> Result<(), SendError<Message>> {
    self.tx.send(Message::ControllerRequest(req)).await
  }

  pub async fn recv_req(&self) -> Option<ControllerRequest> {
    if let Some(Message::ControllerRequest(req)) = self.rx.lock().await.recv().await {
      Some(req)
    } else {
      None
    }
  }
}

pub type HostSessionStorage = StateMap<String, HostSession>;
pub trait HostSessionStorageExt {
  fn send_request(
    &self, id: &String, req: ControllerRequestPayload,
  ) -> impl std::future::Future<Output = Option<Result<u32, SendError<Message>>>> + Send;
}

impl HostSessionStorageExt for HostSessionStorage {
  async fn send_request(&self, id: &String, req: ControllerRequestPayload) -> Option<Result<u32, SendError<Message>>> {
    if let Some(session) = self.get_arc(id) {
      debug!("Sending request to session: {}", session.host_id);
      let task_id: u32 = rand::random::<u32>();
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
        session.tasks.insert(task_id, None);
        Some(Ok(task_id))
      }
    } else {
      None
    }
  }
}
