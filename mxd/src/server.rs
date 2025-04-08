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
use log::{debug, error, info, trace, warn};
use serde::Serialize;
use tokio::{net::TcpListener, select};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

use crate::{
    Cli, api, srv,
    states::{SharedAppState, host_session::HostSession, new_shared_app_state},
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

pub(crate) async fn main(config: Cli) -> Result<()> {
    let halt_singal = CancellationToken::new();
    let halt_singal2 = halt_singal.clone();
    let app: SharedAppState = new_shared_app_state();
    let serve = axum::serve(
        TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            config.port,
        ))
        .await?,
        Router::new()
            .route("/ws", get(handle_ws).head(async || StatusCode::OK))
            .nest("/api", api::build(app.clone(), config.apikey))
            .nest("/srv", srv::file::build(app.clone()))
            .nest_service("/static", ServeDir::new(config.static_path))
            .with_state(app.clone())
            .into_make_service_with_connect_info::<SocketConnectInfo>(),
    )
    .with_graceful_shutdown(async move {
        select! {
            _ = halt_singal.cancelled() => {
                info!("Server shutting down");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, shutting down");
            }
        }
    });
    info!("Server started on {}", serve.local_addr()?);

    tokio::spawn(lifetime_helper(app.clone(), halt_singal2.clone()));
    serve.await?;
    info!("Server stopping");
    halt_singal2.cancel();
    halt_singal2.cancelled().await;
    info!("Server stopped");
    Ok(())
}

async fn lifetime_helper(app: SharedAppState, halt_signal: CancellationToken) {
    let cancellation_token = halt_signal.child_token(); // to avoid unused variable warning, maybe used in the future
    loop {
        select! {
            _ = cancellation_token.cancelled() => {
                debug!("lifetime helper is shutting down");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                trace!("Performing periodic tasks");
                helper_heartbeat(app.clone()).await;
            }
        }
    }
}

async fn helper_heartbeat(app: SharedAppState) {
    let _ = app.host_session.list_sessions().await;
}

async fn handle_ws(
    State(app): State<SharedAppState>,
    ConnectInfo(socket_info): ConnectInfo<SocketConnectInfo>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("WebSocket connection with {:?}", socket_info);
    match handle_ws_inner(app, socket_info, headers, ws).await {
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
) -> Result<Response> {
    let params: ConnectHandshake = ConnectHandshake::from_str(
        headers
            .get(CONNECT_HANDSHAKE_HEADER_KEY)
            .ok_or(anyhow!("Missing handshake header"))?
            .to_str()?,
    )?;
    Ok(ws.on_upgrade(async move |socket| {
        if let Err(e) = handle_socket(socket, params.clone(), socket_info, app.clone()).await {
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
) -> Result<()> {
    info!("WebSocket connection for id: {}", params.host_id);
    let session = app
        .host_session
        .resume_session(&params.host_id)
        .await
        .ok_or(anyhow::anyhow!(
            "Failed to obtain session for id: {}",
            params.host_id
        ))?;

    {
        let mut lock = session.extra.lock().await;
        lock.socket_info = Some(socket_info);
        lock.controller_url = Some(params.controller_url);
        lock.system_info = Some(params.system_info);
    }

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

    app.host_session.remove_session(&params.host_id).await; // usually it should remove the closing session
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
