use common::{
    protocol::{
        controller::{
            AgentMessage,
            AgentResponse,
            AgentResponsePayload,
            // CLOSE_CODE, CLOSE_MXA_SHUTDOWN,
            ControllerMessage,
            ControllerRequest,
            PROTOCOL_VERSION,
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

use crate::{discovery::discover_controller, executor::handle_event};

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

async fn safe_sleep(duration: u64) -> bool {
    select! {
        _ = tokio::time::sleep(std::time::Duration::from_millis(duration)) => {
            false
        },
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl-C, shutting down");
            true
        }
    }
}

pub(crate) async fn handle_ws_url(
    env_ws_url: Option<String>,
    host_id: String,
    session_id: String,
    envs: Vec<String>,
) -> Result<bool> {
    loop {
        let ws_url = if let Some(env_ws_url) = env_ws_url.clone() {
            info!(
                "Using controller URL from environment variable: {}",
                &env_ws_url
            );
            env_ws_url
        } else {
            info!("Discovering controller URL...");
            let discovery_response = select! {
                r = discover_controller() => r,
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C, canceling discovery and exit");
                    return Ok(true);
                }
            };
            let controllers = match discovery_response {
                Ok(c) => c,
                Err(err) => {
                    error!("Failed to discover controller: {}", err);
                    std::process::exit(1);
                }
            };
            if controllers.is_empty() {
                warn!("No controller discovered");
                return Err(anyhow!("Failed to discover controller"));
            } else {
                controllers[0].clone()
            }
        };
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

        let mut retry = 0;
        while retry < 5 {
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
                    retry = 0;
                    match handle_conn(ws).await {
                        Err(e) => {
                            error!("Failed to handle connection: {}", e);
                            continue;
                        }
                        Ok(exit) => {
                            info!("Exiting connection loop");
                            if exit {
                                return Ok(true);
                            }
                        }
                    }
                    warn!("Connection closed");
                    if safe_sleep(5000).await {
                        return Ok(true);
                    }
                    break;
                }
                Err(err) => {
                    error!("Failed to connect to controller: {}", err);
                    if safe_sleep(((1.5f32).powi(retry) * 3000f32 + 5000f32) as u64).await {
                        return Ok(true);
                    }
                    retry += 1
                }
            }
        }
        info!("Retrying connection to controller...");
    }
}

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<bool> {
    let (mut tx, mut rx) = ws.split();
    let (tx_tx, mut tx_rx) = mpsc::channel::<Message>(16);
    debug!("Websocket connected to controller. Begin to handle message loop");
    loop {
        select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, shutting down websocket connection");
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
                        }
                        break Ok(c);
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
        Ok(false)
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
            // return Ok(e
            //     .map(|v| u16::from(v.code) == CLOSE_CODE && v.reason == CLOSE_MXA_SHUTDOWN)
            //     .unwrap_or(false));
            return Ok(false);
        }
        Message::Frame(_) => warn!("Received a malformed message from controller, ignored",),
    }
    Ok(false)
}
