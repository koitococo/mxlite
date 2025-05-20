use std::{
  str::FromStr,
  sync::Arc,
  time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use axum::{
  extract::{
    State,
    connect_info::ConnectInfo,
    ws::{Message, WebSocket, WebSocketUpgrade},
  },
  http::{HeaderMap, StatusCode},
  response::{IntoResponse, Response},
};
use common::protocol::{
  messaging::Message as ProtocolMessage,
  handshake::{CONNECT_HANDSHAKE_HEADER_KEY, ConnectHandshake},
};
use futures_util::SinkExt;
use log::{debug, error, info, warn};
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::states::{
  SharedAppState,
  host_session::{ExtraInfo, HostSession},
};

use super::SocketConnectInfo;

pub(super) async fn handle_ws(
  State(app): State<SharedAppState>, ConnectInfo(socket_info): ConnectInfo<SocketConnectInfo>, headers: HeaderMap,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  info!("WebSocket connection with {socket_info:?}");
  let ct = app.cancel_signal.child_token();
  match handle_ws_inner(app, socket_info, headers, ws, ct).await {
    Ok(ws) => ws,
    Err(e) => {
      error!("Failed to handle WebSocket connection: {e}");
      (StatusCode::BAD_REQUEST, "Bad Request").into_response()
    }
  }
}

async fn handle_ws_inner(
  app: SharedAppState, socket_info: SocketConnectInfo, headers: HeaderMap, ws: WebSocketUpgrade, ct: CancellationToken,
) -> Result<Response> {
  let params: ConnectHandshake = ConnectHandshake::from_str(
    headers.get(CONNECT_HANDSHAKE_HEADER_KEY).ok_or(anyhow!("Missing handshake header"))?.to_str()?,
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
    app.host_session.remove(&params.host_id); // usually it should remove the closing session
  }))
}

// Function to handle the WebSocket connection
async fn handle_socket(
  mut ws: WebSocket, params: ConnectHandshake, socket_info: SocketConnectInfo, app: SharedAppState,
  ct: CancellationToken,
) -> Result<()> {
  info!("WebSocket connection for id: {} {}", params.host_id, params.session_id);
  let session = app
    .host_session
    .create_session(
      &params.host_id,
      ExtraInfo {
        socket_info,
        controller_url: params.controller_url,
        system_info: params.system_info,
        envs: params.envs,
        session_id: params.session_id.clone(),
      },
    )
    .ok_or(anyhow::anyhow!("Failed to obtain session for id: {}", params.host_id))?;
  if session.session_id != params.session_id {
    error!(
      "Session ID mismatch: expected {}, got {}",
      session.session_id, params.session_id
    );
    session.notify.notify_waiters();
    return Err(anyhow!("Session ID mismatch"));
  }
  let mut last_seen = Instant::now();
  loop {
    select! {
        _ = ct.cancelled() => {
            warn!("WebSocket connection cancelled for id: {}", params.host_id);
            break;
        }
        _ = session.notify.notified() => {
            info!("Session notified for id: {}", params.host_id);
            break;
        }
        req = session.recv_req() => {
            if let Some(req) = req {
                debug!("Sending request: {req:?}");
                ws.send(ProtocolMessage::ControllerRequest(req).to_string().into()).await?;
            } else {
                info!("Internal channel closed for id: {}", params.host_id);
                break;
            }
        }
        r = handle_ws_recv(&mut ws, session.clone()) => {
            last_seen = Instant::now();
            match r {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    error!("Failed to handle WebSocket message: {e}");
                }
            }
        }
        _ = sleep(Duration::from_secs(15)) => {
            if last_seen.elapsed() > Duration::from_secs(20) {
                warn!("WebSocket connection timed out for id: {}", params.host_id);
            }
            if last_seen.elapsed() > Duration::from_secs(60) {
                error!("WebSocket connection closed due to inactivity for id: {}", params.host_id);
                break;
            }
            if let Err(e) = ws.send(Message::Ping("ping".into())).await {
                error!("Failed to send ping: {e}");
                break;
            }
        }
    }
  }

  ws.close().await?;
  Ok(())
}

async fn handle_ws_recv(ws: &mut WebSocket, session: Arc<HostSession>) -> Result<bool> {
  if let Some(msg) = ws.recv().await {
    let msg = msg?;
    match msg {
      Message::Text(data) => {
        let data = data.to_string();
        let msg = ProtocolMessage::from_str(&data)?;
        tokio::spawn(super::collector::handle_msg(msg, session));
        Ok(true)
      }
      Message::Binary(_) => Err(anyhow!("Binary message not supported")), // Not supported yet
      Message::Close(e) => {
        debug!("WebSocket connection closed: {e:?}");
        Ok(false)
      }
      Message::Ping(_) | Message::Pong(_) => Ok(true), // handled by underlying library
    }
  } else {
    warn!("WebSocket connection closed");
    Ok(false)
  }
}
