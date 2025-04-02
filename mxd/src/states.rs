use std::{clone::Clone, sync::Arc};

use anyhow::{Result, anyhow};
use common::{
    messages::{AgentResponse, ControllerMessage, ControllerRequest},
    state::{AtomticStateStorage, StateStorage as _},
    system_info::SystemInfo,
    utils::sha1_for_file,
};
use log::debug;
use serde::Serialize;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender, error::SendError},
};

use crate::server::SocketConnectInfo;

#[derive(Clone)]
pub(crate) enum TaskState {
    Pending,
    Finished(AgentResponse),
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ExtraInfo {
    pub(crate) socket_info: Option<SocketConnectInfo>,
    pub(crate) system_info: Option<SystemInfo>,
}

impl ExtraInfo {
    pub(crate) fn new() -> Self {
        ExtraInfo {
            socket_info: None,
            system_info: None,
        }
    }
}

pub(crate) struct HostSession {
    id: String,
    tx: Sender<ControllerMessage>,
    rx: Mutex<Receiver<ControllerMessage>>,
    tasks: AtomticStateStorage<u64, TaskState>,
    pub(crate) extra: Mutex<ExtraInfo>,
}

impl HostSession {
    pub(crate) fn new(id: String) -> Self {
        let (tx, rx) = mpsc::channel(32);
        HostSession {
            id,
            tx,
            rx: Mutex::new(rx),
            tasks: AtomticStateStorage::new(),
            extra: Mutex::new(ExtraInfo::new()),
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
        loop {
            let id: u64 = rand::random::<u64>() >> 16;
            if self.tasks.add(id, TaskState::Pending).await {
                return id;
            }
        }
    }

    pub(crate) async fn set_task_finished(&self, id: u64, resp: AgentResponse) {
        self.tasks.set(id, TaskState::Finished(resp)).await;
    }

    pub(crate) async fn get_task_state(&self, id: u64) -> Option<Arc<TaskState>> {
        self.tasks.get(&id).await
    }
}

pub(crate) struct FileMap {
    file_path: String,
}

pub(crate) struct AppState {
    host_session: AtomticStateStorage<String, HostSession>,
    file_map: AtomticStateStorage<String, FileMap>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {
            host_session: AtomticStateStorage::new(),
            file_map: AtomticStateStorage::new(),
        }
    }

    pub(crate) async fn resume_session(&self, id: &String) -> Option<Arc<HostSession>> {
        if !self.host_session.has(id).await {
            self.host_session
                .set(id.to_string(), HostSession::new(id.to_string()))
                .await;
        }
        self.host_session.get(id).await
    }

    pub(crate) async fn remove_session(&self, id: &String) {
        self.host_session.del(id).await;
    }

    pub(crate) async fn list_sessions(&self) -> Vec<String> {
        self.host_session.list().await
    }

    pub(crate) async fn get_extra_info(&self, id: &String) -> Option<ExtraInfo> {
        if let Some(session) = self.host_session.get(id).await {
            let guard = session.extra.lock().await;
            Some(guard.clone())
        } else {
            None
        }
    }

    pub(crate) async fn send_req(
        &self,
        id: &String,
        mut req: ControllerRequest,
    ) -> Option<Result<u64, SendError<ControllerMessage>>> {
        if let Some(session) = self.host_session.get(id).await {
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

    pub(crate) async fn get_resp(&self, id: &String, task_id: u64) -> Option<Option<TaskState>> {
        if let Some(session) = self.host_session.get(id).await {
            Some(
                session
                    .get_task_state(task_id)
                    .await
                    .map(|task| task.as_ref().clone()),
            )
        } else {
            None
        }
    }

    pub(crate) async fn add_file(&self, file: String) -> Result<String> {
        let hash = sha1_for_file(file.as_str()).await?;
        if self
            .file_map
            .add(hash.clone(), FileMap { file_path: file })
            .await
        {
            Ok(hash)
        } else {
            Err(anyhow!("File already exists"))
        }
    }

    pub(crate) async fn get_file(&self, id: &String) -> Option<String> {
        self.file_map
            .get(id)
            .await
            .map(|file_map| file_map.file_path.clone())
    }

    pub(crate) async fn get_all_files(&self) -> Vec<String> {
        self.file_map.list().await
    }
}

pub(crate) type SharedAppState = Arc<AppState>;
pub(crate) fn new_shared_app_state() -> SharedAppState {
    Arc::new(AppState::new())
}
