use std::str::FromStr;

use super::cli::StartupArgs;
use crate::{
  discovery::discover_controller_once,
  protocol::{
    auth::AuthRequest,
    handshake::{ConnectHandshake, CONNECT_AGENT_AUTH_HEADER_KEY, CONNECT_HANDSHAKE_HEADER_KEY},
    messaging::{AgentResponse, Message as ProtocolMessage, PROTOCOL_VERSION},
  },
  system_info::{self},
  utils::{
    hash::sha2_256_for_str, retry::{async_with_retry, Retry, RetryResult}, util::safe_sleep
  },
};

use tokio::{
  net::TcpStream,
  select,
  sync::mpsc::{self, Sender},
};
use url::Url;

use crate::utils::signal::ctrl_c;
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, trace, warn};
use tokio_tungstenite::{
  MaybeTlsStream, WebSocketStream, connect_async_with_config,
  tungstenite::{
    client::IntoClientRequest,
    handshake::client::Response,
    protocol::{Message, WebSocketConfig},
  },
};

use crate::agent::executor::handle_event;

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
  Continue,
}

pub(crate) async fn start_agent(args: StartupArgs) -> Result<()> {
  loop {
    let Some(ws_url) = get_ws_url(&args).await else {
      warn!("No controller URL found");
      continue;
    };
    info!("Connecting to controller websocket: {}", &ws_url);

    match async_with_retry(async || handle_connect(&args, &ws_url).await, 5).await {
      RetryResult::Break => {
        info!("Exiting...");
        break;
      }
      RetryResult::Return(should_break) => {
        if should_break {
          break;
        } else {
          continue;
        }
      }
      RetryResult::NoResult => {
        warn!("Failed to connect to controller");
      }
    }
    safe_sleep(5000).await;
  }
  Ok(())
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

async fn get_ws_url(args: &StartupArgs) -> Option<Url> {
  if let Some(ws_url) = args.ws_url.as_ref() {
    info!("Using controller URL from environment variable: {ws_url}");
    Some(Url::from_str(ws_url.as_str()).unwrap())
  } else {
    info!("Discovering controller URL...");
    let controllers = select! {
      r = discover_controller() => r,
      _ = ctrl_c() => {
        info!("Canceling discovery and exit");
        return None;
      }
    };
    if controllers.is_empty() {
      warn!("No controller discovered");
      return None;
    } else {
      Some(controllers[0].clone())
    }
  }
}

async fn connect_to(
  args: &StartupArgs, ws_url: &Url,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)> {
  let mut req = ws_url.as_str().into_client_request()?;
  let headers = req.headers_mut();
  headers.insert(
    CONNECT_HANDSHAKE_HEADER_KEY,
    (ConnectHandshake {
      version: PROTOCOL_VERSION,
      controller_url: ws_url.clone(),
      host_id: args.host_id.clone(),
      session_id: args.session_id.clone(),
      envs: args.envs.clone(),
      system_info: system_info::collect_info(),
    })
    .to_string()
    .parse()?,
  );
  handle_pre_auth(&args, headers)?;

  connect_async_with_config(req.clone(), Some(WebSocketConfig { ..Default::default() }), false)
    .await
    .map_err(|e| {
      error!("Failed to connect to controller: {e}");
      anyhow!(e)
    })
}

fn handle_pre_auth(args: &StartupArgs, headers: &mut http::HeaderMap) -> Result<()> {
  let (_, privkey) = &args.key_pair;
  let sign = AuthRequest::new_with_privkey_string(privkey)?;
  headers.insert(CONNECT_AGENT_AUTH_HEADER_KEY, sign.encode().parse()?);
  Ok(())
}

fn handle_post_auth(args: &StartupArgs, resp: &Response) -> bool {
  let headers = resp.headers();
  let Some(auth_header) = headers.get(CONNECT_AGENT_AUTH_HEADER_KEY) else {
    warn!("No authentication header found in response");
    return !args.enforce_auth;
  };
  let Ok(header_val) = auth_header.to_str() else {
    error!("Failed to convert authentication header to string");
    return false;
  };
  let Ok(auth_req) = AuthRequest::decode(header_val) else {
    error!("Failed to decode authentication header");
    return false;
  };
  if !auth_req.verify() {
    error!("Authentication failed, controller is not trusted");
    return false;
  }
  let pubkey = auth_req.encoded_pubkey();
  if args.enforce_auth {
    let Ok(hashed) = sha2_256_for_str(&pubkey) else {
      error!("Failed to hash public key");
      return false;
    };
    return args.trusted_controllers.contains(&hashed)
  }
  true
}

/// Returns a boolean indicating whether the loop should be break
/// If `None` is returned, it means a error occurred and the loop should continue after sleep
async fn handle_connect(args: &StartupArgs, ws_url: &Url) -> Retry<bool> {
  match connect_to(args, ws_url).await {
    Ok((ws, resp)) => {
      if !handle_post_auth(args, &resp) {
        error!("Authentication failed, exiting");
        return Retry::Return(false);
      }
      info!("Connected to controller");
      match handle_conn(ws).await {
        Err(e) => {
          error!("Failed to handle connection: {e}");
          Retry::RetryImmediate
        }
        Ok(exit) => match exit {
          BreakLoopReason::LostConnection => {
            error!("Lost connection to controller");
            Retry::RetryWithDelay
          }
          BreakLoopReason::Shutdown => {
            info!("Shutting down");
            Retry::Return(true)
          }
          _ => Retry::RetryImmediate,
        },
      }
    }
    Err(err) => {
      error!("Failed to connect to controller: {err}");
      Retry::RetryWithDelay
    }
  }
}

async fn handle_conn(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Result<BreakLoopReason> {
  let (mut tx, mut rx) = ws.split();
  let (tx_tx, mut tx_rx) = mpsc::channel::<Message>(16);
  debug!("Websocket connected to controller. Begin to handle message loop");
  loop {
    select! {
      _ = ctrl_c() => {
        info!("Shutting down websocket connection");
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
  Ok(BreakLoopReason::Continue)
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
