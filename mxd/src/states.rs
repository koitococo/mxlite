use std::{collections::BTreeMap, sync::Arc};

use common::messages::{AgentResponse, ControllerMessage, ControllerRequest};
use log::debug;
use serde::Serialize;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender, error::SendError},
};

use crate::server::SocketConnectInfo;

pub(crate) enum TaskState {
    Pending,
    Finished(AgentResponse),
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ExtraInfo {
    pub(crate) socket_info: Option<SocketConnectInfo>
}

pub(crate) struct Session {
    id: String,
    tx: Sender<ControllerMessage>,
    rx: Mutex<Receiver<ControllerMessage>>,
    tasks: Mutex<BTreeMap<u64, TaskState>>,
    pub(crate) extra: Mutex<ExtraInfo>,
}

impl ExtraInfo {
    pub(crate) fn new() -> Self {
        ExtraInfo {
            socket_info: None,
        }
    }
}

impl Session {
    pub(crate) fn new(id: String) -> Self {
        let (tx, rx) = mpsc::channel(32);
        Session {
            id,
            tx,
            rx: Mutex::new(rx),
            tasks: Mutex::new(BTreeMap::new()),
            extra: Mutex::new(ExtraInfo::new())
        }
    }

    pub(crate) async fn send_req(
        &self,
        req: ControllerRequest,
    ) -> Result<(), SendError<ControllerMessage>> {
        self.tx.send(ControllerMessage { request: req }).await
    }

    pub(crate) async fn recv_req(&self) -> Option<ControllerMessage> {
        self.rx.lock().await.recv().await
    }

    pub(crate) async fn new_task(&self) -> u64 {
        let mut guard = self.tasks.lock().await;
        loop {
            let id: u64 = rand::random();
            let id = id >> 16;
            if !guard.contains_key(&id) {
                guard.insert(id, TaskState::Pending);
                return id;
            }
        }
    }

    pub(crate) async fn set_task_finished(&self, id: u64, resp: AgentResponse) {
        let mut guard = self.tasks.lock().await;
        if let Some(task) = guard.get_mut(&id) {
            *task = TaskState::Finished(resp);
        }
    }

    pub(crate) async fn get_task_state(&self, id: u64) -> Option<TaskState> {
        let mut guard = self.tasks.lock().await;
        let r = guard.get(&id);
        if let Some(task) = r {
            match task {
                TaskState::Pending => Some(TaskState::Pending),
                TaskState::Finished(_) => {
                    guard.remove(&id)
                }
            }
        } else {
            None
        }
    }
}

pub(crate) struct AppState {
    sessions: Mutex<BTreeMap<String, Arc<Session>>>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {
            sessions: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) async fn resume_session(&self, id: &str) -> Option<Arc<Session>> {
        let mut guard = self.sessions.lock().await;
        if !guard.contains_key(id) {
            guard.insert(id.to_string(), Arc::new(Session::new(id.to_string())));
        }
        guard.get(id).cloned()
    }

    pub(crate) async fn remove_session(&self, id: &str) {
        self.sessions.lock().await.remove(id);
    }

    pub(crate) async fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().await.keys().cloned().collect()
    }

    pub(crate) async fn get_extra_info(&self, id: &str) -> Option<ExtraInfo> {
        if let Some(session) = self.find_session(id).await {
            let guard = session.extra.lock().await;
            Some(guard.clone())
        } else {
            None
        }
    }

    async fn find_session(&self, id: &str) -> Option<Arc<Session>> {
        let guard = self.sessions.lock().await;
        guard.get(id).cloned()
    }

    pub(crate) async fn send_req(
        &self,
        id: &str,
        mut req: ControllerRequest,
    ) -> Option<Result<u64, SendError<ControllerMessage>>> {
        if let Some(session) = self.find_session(id).await {
            debug!("Sending request to session: {}", session.id);
            let task_id = session.new_task().await;
            req.id = task_id;
            if let Err(e) = session.send_req(req).await {
                Some(Err(e))
            } else {
                Some(Ok(task_id))
            }
        } else {
            None
        }
    }

    pub(crate) async fn get_resp(&self, id: &str, task_id: u64) -> Option<Option<TaskState>> {
        if let Some(session) = self.find_session(id).await {
            Some(session.get_task_state(task_id).await)
        } else {
            None
        }
    }
}

pub(crate) type SharedAppState = Arc<AppState>;
pub(crate) fn new_shared_app_state() -> SharedAppState {
    Arc::new(AppState::new())
}
