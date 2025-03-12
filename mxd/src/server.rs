use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use common::messages::AgentMessage;
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::{net::TcpListener, select};

use crate::{
    api::build_api,
    states::{Session, SharedAppState, new_shared_app_state},
};

pub(crate) async fn main(apikey: String) -> Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
    info!("Server listening on {}", addr);

    let app: SharedAppState = new_shared_app_state();
    axum::serve(
        TcpListener::bind(addr).await?,
        Router::new()
            .route("/ws", get(handle_ws).head(async || StatusCode::OK))
            .nest("/api", build_api(app.clone(), apikey))
            .with_state(app.clone()),
    )
    .await?;
    Ok(())
}

#[derive(Deserialize)]
struct WsParams {
    host_id: String,
}

async fn handle_ws(
    State(app): State<SharedAppState>,
    params: Query<WsParams>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let id = params.host_id.clone();
    ws.on_upgrade(async move |socket| {
        if let Err(e) = handle_socket(socket, id.clone(), app.clone()).await {
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
async fn handle_socket(mut ws: WebSocket, id: String, app: SharedAppState) -> Result<()> {
    info!("WebSocket connection for id: {}", id);
    let session = app
        .resume_session(&id)
        .await
        .ok_or(anyhow::anyhow!("Failed to obtain session for id: {}", id))?;

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
