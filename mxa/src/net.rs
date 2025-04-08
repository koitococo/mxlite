use common::{
    messages::{
        AgentMessage, AgentResponse, AgentResponsePayload, CONNECT_HANDSHAKE_HEADER_KEY,
        ConnectHandshake, ControllerMessage, ControllerRequest,
    },
    system_info::SystemInfo,
};
use std::{str::FromStr, sync::Arc};
use tokio::{net::TcpStream, select, sync::Mutex};

use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use log::{debug, error, info, trace, warn};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        client::IntoClientRequest,
        protocol::{Message, WebSocketConfig},
    },
};

use crate::executor::handle_event;

#[derive(Debug, Clone)]
struct RespondHandler {
    tx: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
}

impl RespondHandler {
    fn new(tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> Self {
        RespondHandler {
            tx: Arc::new(Mutex::new(tx)),
        }
    }

    async fn respond(self, msg: Message) -> Result<()> {
        let mut guard = self.tx.lock().await;
        guard.send(msg).await?;
        Ok(())
    }

    async fn ping(self) -> Result<()> {
        self.respond(Message::Ping(vec![])).await
    }

    async fn pong(self, data: Vec<u8>) -> Result<()> {
        self.respond(Message::Pong(data)).await
    }
}

pub(crate) struct Context {
    pub(crate) request: ControllerRequest,
    responder: RespondHandler,
}

impl Context {
    pub(crate) async fn respond2(&self, ok: bool, payload: AgentResponsePayload) {
        if let Err(e) = self
            .responder
            .clone()
            .respond(Message::Text(
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
) -> Result<()> {
    info!("Use Controller URL: {}", &ws_url);
    loop {
        info!("Connecting to controller websocket: {}", &ws_url);
        let mut req = ws_url.clone().into_client_request()?;
        req.headers_mut().insert(
            CONNECT_HANDSHAKE_HEADER_KEY,
            (ConnectHandshake {
                version: common::messages::PROTOCOL_VERSION,
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
                    handle_conn(ws).await?;
                    warn!("Connection closed");
                    break;
                }
                Err(err) => {
                    error!("Failed to connect to controller: {}", err);
                    tokio::time::sleep(std::time::Duration::from_secs(
                        ((1.5f32).powi(retry) * 3f32 + 5f32) as u64,
                    ))
                    .await;
                }
            }
        }
    }
}

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<()> {
    let (tx, mut rx) = ws.split();
    let responder = RespondHandler::new(tx);
    trace!("Websocket connected to controller. Begin to handle message loop");
    loop {
        select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                trace!("Sending ping to controller");
                if let Err(e) = responder.clone().ping().await {
                    error!("Failed to send ping: {}", e);
                    break;
                }
            }
            msg = rx.next() => {
                match handle_ws_event(msg, responder.clone()).await {
                    Ok(c) => {
                        if !c {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to handle WebSocket event: {}", e);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_ws_event(
    event: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    responder: RespondHandler,
) -> Result<bool> {
    if let Some(event) = event {
        match event {
            Ok(ws_msg) => match handle_msg(ws_msg, responder).await {
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

async fn handle_msg(ws_msg: Message, responder: RespondHandler) -> Result<bool> {
    debug!("Received message: {:?}", ws_msg);
    match ws_msg {
        Message::Text(msg) => {
            trace!("Received text message from controller");
            match ControllerMessage::from_str(msg.as_str()) {
                Ok(event_msg) => {
                    info!("Received event: {:?}", event_msg);
                    let ctx = Context {
                        request: event_msg.request,
                        responder,
                    };
                    tokio::spawn(async move {
                        if let Err(e) = handle_event(ctx).await {
                            error!("Failed to handle event: {}", e);
                        }
                    });
                }
                Err(err) => {
                    error!("Failed to parse message: {}", err);
                    if let Err(e) = responder
                        .respond(Message::Text(
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
            responder.pong(f).await?;
        }
        Message::Pong(_) => trace!("Received Pong frame"),
        Message::Close(_) => warn!("Connection is closing"),
        Message::Frame(_) => warn!("Received a malformed message from controller, ignored",),
    }
    Ok(true)
}
