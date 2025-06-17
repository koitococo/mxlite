use std::str::FromStr;

use crate::{
  discovery::discover_controller_once,
  protocol::{
    handshake::{CONNECT_HANDSHAKE_HEADER_KEY, ConnectHandshake},
    messaging::{AgentResponse, Message as ProtocolMessage, PROTOCOL_VERSION},
  },
  system_info::{self},
};
use tokio::{
  net::TcpStream,
  select,
  sync::mpsc::{self, Sender},
};
use url::Url;

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

use crate::agent::{executor::handle_event, utils::safe_sleep};

pub(crate) type MessageSender = Sender<Message>;
pub(crate) trait MessageSend<T> {
  fn send_msg(&self, msg: T) -> bool;
}

impl MessageSend<Message> for MessageSender {
  fn send_msg(&self, msg: Message) -> bool {
    if let Err(e) = self.try_send(msg) {
      error!("Failed to send message: {e}");
      false
    } else {
      true
    }
  }
}

impl MessageSend<String> for MessageSender {
  fn send_msg(&self, msg: String) -> bool { self.send_msg(Message::Text(msg)) }
}

impl MessageSend<ProtocolMessage> for MessageSender {
  fn send_msg(&self, msg: ProtocolMessage) -> bool {
    let Ok(msg): Result<String, _> = msg.try_into() else {
      error!("Failed to convert ProtocolMessage to Message");
      return false;
    };
    self.send_msg(msg)
  }
}

impl MessageSend<AgentResponse> for MessageSender {
  fn send_msg(&self, msg: AgentResponse) -> bool {
    let msg: ProtocolMessage = msg.into();
    self.send_msg(msg)
  }
}

enum BreakLoopReason {
  LostConnection,
  Shutdown,
  ErrorCaptured,
  Nonbreak,
}

async fn discover_controller() -> Vec<Url> {
  loop {
    match discover_controller_once().await {
      Ok(r) => return r,
      Err(e) => {
        error!("Failed to discover controller: {e}");
        if safe_sleep(5000).await {
          return vec![];
        }
      }
    }
  }
}

pub(crate) async fn handle_ws_url(
  env_ws_url: Option<String>, host_id: String, session_id: String, envs: Vec<String>,
) -> Result<bool> {
  loop {
    let ws_url: Url = if let Some(env_ws_url) = env_ws_url.as_ref() {
      info!("Using controller URL from environment variable: {env_ws_url}");
      Url::from_str(env_ws_url.as_str()).unwrap()
    } else {
      info!("Discovering controller URL...");
      let controllers = select! {
        r = discover_controller() => r,
        _ = tokio::signal::ctrl_c() => {
          info!("Received Ctrl-C, canceling discovery and exit");
          return Ok(true);
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

    let mut req = ws_url.as_str().into_client_request()?;
    req.headers_mut().insert(
      CONNECT_HANDSHAKE_HEADER_KEY,
      (ConnectHandshake {
        version: PROTOCOL_VERSION,
        controller_url: ws_url,
        host_id: host_id.clone(),
        session_id: session_id.clone(),
        envs: envs.clone(),
        system_info: system_info::collect_info(),
      })
      .to_string()
      .parse()?,
    );

    let mut retry = 0;
    while retry < 5 {
      match connect_async_with_config(req.clone(), Some(WebSocketConfig { ..Default::default() }), false).await {
        Ok((ws, _)) => {
          info!("Connected to controller");
          retry = 0;
          match handle_conn(ws).await {
            Err(e) => {
              error!("Failed to handle connection: {e}");
              continue;
            }
            Ok(exit) => match exit {
              BreakLoopReason::LostConnection => {
                error!("Lost connection to controller");
              }
              BreakLoopReason::Shutdown => {
                info!("Shutting down");
                return Ok(true);
              }
              _ => (),
            },
          }
          warn!("Connection closed");
          if safe_sleep(5000).await {
            return Ok(true);
          }
          break;
        }
        Err(err) => {
          error!("Failed to connect to controller: {err}");
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

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<BreakLoopReason> {
  let (mut tx, mut rx) = ws.split();
  let (tx_tx, mut tx_rx) = mpsc::channel::<Message>(16);
  debug!("Websocket connected to controller. Begin to handle message loop");
  loop {
    select! {
      _ = tokio::signal::ctrl_c() => {
        info!("Received Ctrl-C, shutting down websocket connection");
        tx.send(Message::Close(None)).await?;
        break Ok(BreakLoopReason::Shutdown);
      }
      _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
        trace!("Sending ping to controller");
        if let Err(e) = tx.send(Message::Ping("ping".into())).await {
          error!("Failed to send ping: {e}");
          break Ok(BreakLoopReason::LostConnection);
        }
      }
      msg = rx.next() => {
        match handle_ws_message(msg, tx_tx.clone()).await {
          Ok(BreakLoopReason::LostConnection) => {
            error!("Lost connection to controller");
            break Ok(BreakLoopReason::LostConnection);
          }
          Ok(_) => { continue }
          Err(e) => {
              error!("Failed to handle WebSocket event: {e}");
              break Ok(BreakLoopReason::ErrorCaptured);
          }
        }
      }
      msg = tx_rx.recv() => {
        if let Some(msg) = msg {
            debug!("Sending message to controller: {msg:?}");
            if let Err(e) = tx.send(msg).await {
                error!("Failed to send message to controller: {e}");
                break Ok(BreakLoopReason::ErrorCaptured);
            }
        } else {
          info!("Internal channel closed");
          break Ok(BreakLoopReason::Shutdown);
        }
      }
    }
  }
}

async fn handle_ws_message(
  event: Option<Result<Message, tokio_tungstenite::tungstenite::Error>>, tx: Sender<Message>,
) -> Result<BreakLoopReason> {
  if let Some(event) = event {
    match event {
      Ok(ws_msg) => match handle_msg(ws_msg, tx).await {
        Ok(c) => Ok(c),
        Err(e) => {
          error!("Failed to handle message: {e}");
          Err(e)
        }
      },
      Err(err) => {
        error!("Failed to receive message: {err}");
        Err(anyhow!(err))
      }
    }
  } else {
    Ok(BreakLoopReason::ErrorCaptured)
  }
}

async fn handle_msg(msg: Message, tx: Sender<Message>) -> Result<BreakLoopReason> {
  match msg {
    Message::Text(msg) => {
      trace!("Received text message from controller");
      handle_text_msg(msg, tx);
    }
    Message::Binary(_) => {
      warn!("Received binary message from controller, which is not supported");
    }
    Message::Ping(f) => {
      trace!("Received Ping frame");
      tx.send(Message::Pong(f)).await?;
    }
    Message::Pong(_) => trace!("Received Pong frame"),
    Message::Close(e) => {
      warn!("Connection is closing: {e:?}");
      // return Ok(e
      //     .map(|v| u16::from(v.code) == CLOSE_CODE && v.reason == CLOSE_MXA_SHUTDOWN)
      //     .unwrap_or(false));
      return Ok(BreakLoopReason::LostConnection);
    }
    Message::Frame(_) => warn!("Received a malformed message from controller, ignored",),
  }
  Ok(BreakLoopReason::Nonbreak)
}

fn handle_text_msg(msg: String, tx: Sender<Message>) {
  match ProtocolMessage::try_from(msg.as_str()) {
    Ok(ProtocolMessage::ControllerRequest(request)) => {
      info!("Received event: {request:?}");
      tokio::spawn(async move { handle_event(request, tx).await });
    }
    Ok(_) => {
      warn!("Received unsupported message type, ignoring: {msg}");
      tx.send_msg(ProtocolMessage::None);
    }
    Err(err) => {
      error!("Failed to parse message: {err}; dropping message");
      debug!("Message content: {msg}");
      tx.send_msg(ProtocolMessage::None);
    }
  }
}
