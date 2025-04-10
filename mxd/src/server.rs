use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use axum::{
    Router,
    extract::{
        State,
        connect_info::{ConnectInfo, Connected},
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    serve::IncomingStream,
};
use common::protocol::{
    controller::AgentMessage,
    handshake::{CONNECT_HANDSHAKE_HEADER_KEY, ConnectHandshake},
};
use futures_util::SinkExt;
use log::{debug, error, info, warn};
use serde::Serialize;
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

use crate::{
    StartupArguments, api, srv,
    states::{AppState, SharedAppState, host_session::HostSession},
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

pub(crate) async fn main(config: StartupArguments) -> Result<()> {
    let halt_signal = CancellationToken::new();
    let halt_signal2 = halt_signal.clone();
    let app: SharedAppState = Arc::new(AppState::new(halt_signal.clone(), config.clone()));
    let serve = axum::serve(
        TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            config.port,
        ))
        .await?,
        Router::new()
            .route("/ws", get(handle_ws).head(async || StatusCode::OK))
            .nest("/api", api::build(app.clone()))
            .nest("/srv", srv::build(app.clone()))
            .nest_service("/static", ServeDir::new(config.static_path))
            .with_state(app.clone())
            .into_make_service_with_connect_info::<SocketConnectInfo>(),
    )
    .with_graceful_shutdown(async move {
        select! {
            _ = halt_signal.cancelled() => {
                info!("Server shutting down");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, shutting down");
            }
        }
    });
    info!("Server started on {}", serve.local_addr()?);

    tokio::spawn(lifetime_helper(app.clone(), halt_signal2.clone()));
    serve.await?;
    info!("Server stopping");
    halt_signal2.cancel();
    halt_signal2.cancelled().await;
    info!("Server stopped");
    Ok(())
}

async fn lifetime_helper(_app: SharedAppState, halt_signal: CancellationToken) {
    let cancellation_token = halt_signal.child_token(); // to avoid unused variable warning, maybe used in the future
    loop {
        select! {
            _ = cancellation_token.cancelled() => {
                debug!("lifetime helper is shutting down");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                // trace!("Performing periodic tasks");
                // helper_heartbeat(app.clone()).await;
            }
        }
    }
}

// async fn helper_heartbeat(app: SharedAppState) {
//     let _ = app.host_session.list_sessions().await;
// }

async fn handle_ws(
    State(app): State<SharedAppState>,
    ConnectInfo(socket_info): ConnectInfo<SocketConnectInfo>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("WebSocket connection with {:?}", socket_info);
    let ct = (&app).cancel_signal.child_token();
    match handle_ws_inner(app, socket_info, headers, ws, ct).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("Failed to handle WebSocket connection: {}", e);
            (StatusCode::BAD_REQUEST, "Bad Request").into_response()
        }
    }
}

async fn handle_ws_inner(
    app: SharedAppState,
    socket_info: SocketConnectInfo,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    ct: CancellationToken,
) -> Result<Response> {
    let params: ConnectHandshake = ConnectHandshake::from_str(
        headers
            .get(CONNECT_HANDSHAKE_HEADER_KEY)
            .ok_or(anyhow!("Missing handshake header"))?
            .to_str()?,
    )?;
    Ok(ws.on_upgrade(async move |socket| {
        if let Err(e) = handle_socket(socket, params.clone(), socket_info, app.clone(), ct).await {
            error!(
                "Failed to handle WebSocket connection for host {}: {}",
                params.host_id, e
            );
        } else {
            info!("WebSocket connection closed for id: {}", params.host_id);
        }
    }))
}

// Function to handle the WebSocket connection
async fn handle_socket(
    mut ws: WebSocket,
    params: ConnectHandshake,
    socket_info: SocketConnectInfo,
    app: SharedAppState,
    ct: CancellationToken,
) -> Result<()> {
    info!("WebSocket connection for id: {}", params.host_id);
    let session = app
        .host_session
        .resume_session(&params.host_id, &params.session_id)
        .await
        .ok_or(anyhow::anyhow!(
            "Failed to obtain session for id: {}",
            params.host_id
        ))?;
    if session.session_id != params.session_id {
        error!(
            "Session ID mismatch: expected {}, got {}",
            session.session_id, params.session_id
        );
        app.host_session.remove_session(&params.host_id).await;
        return Err(anyhow!("Session ID mismatch"));
    }
    {
        let mut lock = session.extra.lock().await;
        lock.socket_info = Some(socket_info);
        lock.controller_url = Some(params.controller_url);
        lock.system_info = Some(params.system_info);
        lock.envs = Some(params.envs);
    }

    loop {
        select! {
            _ = ct.cancelled() => {
                info!("WebSocket connection cancelled for id: {}", params.host_id);
                break;
            }
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
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                if let Err(e) = ws.send(Message::Ping("ping".into())).await {
                    error!("Failed to send ping: {}", e);
                    break;
                }
            }
        }
    }

    app.host_session.remove_session(&params.host_id).await; // usually it should remove the closing session
    ws.close().await?;
    Ok(())
}

async fn handle_ws_recv(ws: &mut WebSocket, session: Arc<HostSession>) -> Result<bool> {
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

async fn handle_msg(msg: AgentMessage, session: Arc<HostSession>) -> Result<()> {
    debug!("Received message: {:?}", msg);
    if let Some(response) = msg.response {
        session.set_task_finished(response.id, response);
    }
    if let Some(events) = msg.events {
        warn!("Received events: {:?}", events);
    }
    Ok(())
}
