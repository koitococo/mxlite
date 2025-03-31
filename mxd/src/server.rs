use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use axum::{
    Router, extract::{
        Query, State,
        connect_info::{ConnectInfo, Connected},
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    serve::IncomingStream,
};
use common::messages::AgentMessage;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, select};

use crate::{
    api::build_api,
    states::{Session, SharedAppState, new_shared_app_state},
};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SocketConnectInfo {
    pub(crate) local_addr: Option<SocketAddr>,
    pub(crate) remote_addr: Option<SocketAddr>,
}

impl Connected<IncomingStream<'_, TcpListener>> for SocketConnectInfo {
    fn connect_info(target: IncomingStream<'_, TcpListener>) -> Self {
        let io = target.io();
        let local_addr = io.local_addr().ok();
        let remote_addr = io.peer_addr().ok();
        SocketConnectInfo {
            local_addr,
            remote_addr,
        }
    }
}

pub(crate) async fn main(apikey: String) -> Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);

    let app: SharedAppState = new_shared_app_state();
    let serve = axum::serve(
        TcpListener::bind(addr).await?,
        Router::new()
            .route("/ws", get(handle_ws).head(async || StatusCode::OK))
            .nest("/api", build_api(app.clone(), apikey))
            .with_state(app.clone())
            .into_make_service_with_connect_info::<SocketConnectInfo>(),
    );
    info!("Server started on {}", serve.local_addr()?);
    serve.await?;
    Ok(())
}

#[derive(Deserialize)]
struct WsParams {
    host_id: String,
}

async fn handle_ws(
    State(app): State<SharedAppState>,
    ConnectInfo(socket_info): ConnectInfo<SocketConnectInfo>,
    params: Query<WsParams>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("WebSocket connection with {:?}", socket_info);
    let id = params.host_id.clone();
    ws.on_upgrade(async move |socket| {
        if let Err(e) = handle_socket(socket, id.clone(), socket_info, app.clone()).await {
            error!(
                "Failed to handle WebSocket connection for host {}: {}",
                id, e
            );
        } else {
            info!("WebSocket connection closed for id: {}", id);
        }
    })
}

// Function to handle the WebSocket connection
async fn handle_socket(
    mut ws: WebSocket,
    id: String,
    socket_info: SocketConnectInfo,
    app: SharedAppState,
) -> Result<()> {
    info!("WebSocket connection for id: {}", id);
    let session = app
        .resume_session(&id)
        .await
        .ok_or(anyhow::anyhow!("Failed to obtain session for id: {}", id))?;

    let mut lock = session.extra.lock().await;
    lock.socket_info = Some(socket_info);
    drop(lock);

    loop {
        select! {
            req = session.recv_req() => {
                if let Some(req) = req {
                    ws.send(req.to_string().into()).await?;
                } else {
                    break;
                }
            }
            r = handle_ws_recv(&mut ws, session.clone()) => {
                match r {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(e) => {
                        error!("Failed to handle WebSocket message: {}", e);
                    }
                }
            }
        }
    }

    app.remove_session(id.as_str()).await; // usually it should remove the closing session
    Ok(())
}

async fn handle_ws_recv(ws: &mut WebSocket, session: Arc<Session>) -> Result<bool> {
    let msg = ws.recv().await;
    if let Some(msg) = msg {
        let msg = msg?;
        match msg {
            Message::Text(data) => {
                let data = data.to_string();
                let msg = AgentMessage::from_str(&data)?;
                tokio::spawn(handle_msg(msg, session));
                Ok(true)
            }
            Message::Binary(_) => Err(anyhow!("Binary message not supported")), // Not supported yet
            Message::Close(_) => Ok(false),
            Message::Ping(_) | Message::Pong(_) => Ok(true), // handled by underlying library
        }
    } else {
        Ok(false)
    }
}

async fn handle_msg(msg: AgentMessage, session: Arc<Session>) -> Result<()> {
    debug!("Received message: {:?}", msg);
    if let Some(response) = msg.response {
        session.set_task_finished(response.id, response).await;
    }
    if let Some(events) = msg.events {
        warn!("Received events: {:?}", events);
    }
    Ok(())
}
