use common::messages::{
    AgentMessage, AgentResponse, AgentResponsePayload, ControllerMessage, ControllerRequest,
};
use std::{str::FromStr, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};

use crate::executor::handle_event;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use log::{debug, error, info, trace, warn};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};

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
        Message::Ping(msg) => {
            responder.respond(Message::Pong(msg)).await?;
            trace!("Received Ping from controller, Pong sent");
        }
        Message::Pong(_) => trace!("Received Pong from controller"),
        Message::Close(_) => warn!("Websocket connection closed, retry"),
        Message::Frame(_) => warn!("Received a malformed message from controller, ignored",),
    }
    Ok(true)
}

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<()> {
    let (tx, mut rx) = ws.split();
    let responder = RespondHandler::new(tx);
    trace!("Websocket connected to controller. Begin to handle message loop");
    while let Some(event) = rx.next().await {
        match event {
            Ok(ws_msg) => match handle_msg(ws_msg, responder.clone()).await {
                Ok(c) => {
                    if !c {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to handle message: {}", e);
                }
            },
            Err(err) => {
                error!("Failed to receive message: {}", err);
            }
        }
    }
    Ok(())
}

pub(crate) async fn handle_ws_url(ws_url: String, host_id: String) -> Result<()> {
    info!("Use Controller URL: {}", ws_url);
    let ws_url = format!("{}?host_id={}", ws_url, host_id);
    loop {
        let ws_url = ws_url.clone();
        info!("Connecting to controller websocket: {}", ws_url);
        for retry in 0..5 {
            match connect_async(ws_url.clone()).await {
                Ok((ws, _)) => {
                    handle_conn(ws).await?;
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
