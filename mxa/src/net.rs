use common::{
    protocol::{
        controller::{
            AgentMessage, AgentResponse, AgentResponsePayload, ControllerMessage,
            ControllerRequest, PROTOCOL_VERSION,
        },
        handshake::{CONNECT_HANDSHAKE_HEADER_KEY, ConnectHandshake},
    },
    system_info::SystemInfo,
};
use std::str::FromStr;
use tokio::{
    net::TcpStream,
    select,
    sync::mpsc::{self, Sender},
};

use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        client::IntoClientRequest,
        protocol::{Message, WebSocketConfig},
    },
};

use crate::executor::handle_event;

pub(crate) struct Context {
    pub(crate) request: ControllerRequest,
    tx: Sender<Message>,
}

impl Context {
    pub(crate) async fn respond(self, ok: bool, payload: AgentResponsePayload) {
        if let Err(e) = self
            .tx
            .send(Message::Text(
                AgentMessage {
                    response: Some(AgentResponse {
                        id: self.request.id,
                        ok,
                        payload,
                    }),
                    events: None,
                }
                .to_string(),
            ))
            .await
        {
            warn!("Failed to respond request[id={}]: {}", self.request.id, e);
        }
    }
}

pub(crate) async fn handle_ws_url(
    ws_url: String,
    host_id: String,
    session_id: String,
    envs: Vec<String>,
) -> Result<bool> {
    info!("Use Controller URL: {}", &ws_url);
    loop {
        info!("Connecting to controller websocket: {}", &ws_url);
        let mut req = ws_url.clone().into_client_request()?;
        req.headers_mut().insert(
            CONNECT_HANDSHAKE_HEADER_KEY,
            (ConnectHandshake {
                version: PROTOCOL_VERSION,
                controller_url: ws_url.clone(),
                host_id: host_id.clone(),
                session_id: session_id.clone(),
                envs: envs.clone(),
                system_info: SystemInfo::collect_info(),
            })
            .to_string()
            .parse()?,
        );
        for retry in 0..5 {
            match connect_async_with_config(
                req.clone(),
                Some(WebSocketConfig {
                    ..Default::default()
                }),
                false,
            )
            .await
            {
                Ok((ws, _)) => {
                    info!("Connected to controller");
                    match handle_conn(ws).await {
                        Err(e) => {
                            error!("Failed to handle connection: {}", e);
                            continue;
                        }
                        Ok(exit) => {
                            if exit {
                                info!("Exiting connection loop");
                                return Ok(true);
                            }
                        }
                    }
                    warn!("Connection closed");
                    break;
                }
                Err(err) => {
                    error!("Failed to connect to controller: {}", err);
                    tokio::time::sleep(std::time::Duration::from_secs(
                        ((1.5f32).powi(retry) * 3f32 + 5f32) as u64, // 1.5 ^ retry * 3 + 5
                    ))
                    .await;
                }
            }
        }
        info!("Retrying connection to controller...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<bool> {
    let (mut tx, mut rx) = ws.split();
    let (tx_tx, mut tx_rx) = mpsc::channel::<Message>(16);
    debug!("Websocket connected to controller. Begin to handle message loop");
    loop {
        select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, shutting down");
                tx.send(Message::Close(None)).await?;
                break Ok(true);
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                trace!("Sending ping to controller");
                if let Err(e) = tx.send(Message::Ping("ping".into())).await {
                    error!("Failed to send ping: {}", e);
                    break Ok(false);
                }
            }
            msg = rx.next() => {
                match handle_ws_event(msg, tx_tx.clone()).await {
                    Ok(c) => {
                        if c {
                            info!("WebSocket event loop exited");
                            break Ok(true);
                        }
                    }
                    Err(e) => {
                        error!("Failed to handle WebSocket event: {}", e);
                        break Ok(false);
                    }
                }
            }
            msg = tx_rx.recv() => {
                if let Some(msg) = msg {
                    debug!("Sending message to controller: {:?}", msg);
                    if let Err(e) = tx.send(msg).await {
                        error!("Failed to send message to controller: {}", e);
                        break Ok(false);
                    }
                } else {
                    info!("Internal channel closed");
                    break Ok(false);
                }
            }
        }
    }
}

async fn handle_ws_event(
    event: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    tx: Sender<Message>,
) -> Result<bool> {
    if let Some(event) = event {
        match event {
            Ok(ws_msg) => match handle_msg(ws_msg, tx).await {
                Ok(c) => Ok(c),
                Err(e) => {
                    error!("Failed to handle message: {}", e);
                    Err(e)
                }
            },
            Err(err) => {
                error!("Failed to receive message: {}", err);
                Err(anyhow!(err))
            }
        }
    } else {
        Ok(true)
    }
}

async fn handle_msg(ws_msg: Message, tx: Sender<Message>) -> Result<bool> {
    debug!("Received message: {:?}", ws_msg);
    match ws_msg {
        Message::Text(msg) => {
            trace!("Received text message from controller");
            match ControllerMessage::from_str(msg.as_str()) {
                Ok(event_msg) => {
                    info!("Received event: {:?}", event_msg);
                    let ctx = Context {
                        request: event_msg.request,
                        tx,
                    };
                    tokio::spawn(async move {
                        if let Err(e) = handle_event(ctx).await {
                            error!("Failed to handle event: {}", e);
                        }
                    });
                }
                Err(err) => {
                    error!("Failed to parse message: {}", err);
                    if let Err(e) = tx
                        .send(Message::Text(
                            AgentMessage {
                                response: Some(AgentResponse {
                                    id: u64::MAX,
                                    ok: false,
                                    payload: AgentResponsePayload::None,
                                }),
                                events: None,
                            }
                            .to_string(),
                        ))
                        .await
                    {
                        error!("Failed to respond to malformed message: {}", e);
                    }
                }
            }
        }
        Message::Binary(_) => {
            warn!("Received binary message from controller, which is not supported")
        }
        Message::Ping(f) => {
            trace!("Received Ping frame");
            tx.send(Message::Pong(f)).await?;
        }
        Message::Pong(_) => trace!("Received Pong frame"),
        Message::Close(e) => {
            warn!("Connection is closing: {:?}", e);
            tx.send(Message::Close(None)).await?;
            return Ok(true)
        }
        Message::Frame(_) => warn!("Received a malformed message from controller, ignored",),
    }
    Ok(false)
}
