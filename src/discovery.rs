use crate::protocol::discovery::{
  DISCOVERY_PORT, DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE, PROTOCOL_REV,
};
use futures_util::future::join_all;
use log::{debug, error, info, warn};
use reqwest::Url;
use std::{
  net::{IpAddr, Ipv4Addr, SocketAddr},
  str::{self, FromStr},
  time::Duration,
};
use thiserror::Error;
use tokio::{net::UdpSocket, select};

#[derive(Debug, Error)]
pub enum DiscoveryError {
  #[error("No controller found")]
  NoControllerFound,
  #[error("Io Error: {0}")]
  IoError(#[from] std::io::Error),
  #[error("Decode Error: {0}")]
  DecodeError(#[from] std::str::Utf8Error),
  #[error("Protocol Error: {0}")]
  ProtocolError(&'static str),
  #[error("Deserialization Error: {0}")]
  DeserializationError(#[from] serde_json::Error),
}

pub async fn discover_controller_once() -> Result<Vec<Url>, DiscoveryError> {
  info!("Discovering controller");
  let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)).await?;
  socket.set_broadcast(true)?;

  let req = &DiscoveryRequest {
    magic: MAGIC_REQUEST.to_string(),
    revision: PROTOCOL_REV,
  };
  let req_str = req.to_string();
  let req_bin = req_str.as_bytes();
  for _ in 0..10 {
    debug!("Sending discovery request: {req_str}");
    socket
      .send_to(
        req_bin,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), DISCOVERY_PORT),
      )
      .await?;
    select! {
        _ = tokio::time::sleep(Duration::from_secs(3)) => {
          info!("Discovery timeout");
        }
        r = recv_pack(&socket) => {
          if let Some(wss) = handle_pack(r).await {
            return Ok(wss);
          }
        }
    }
  }
  warn!("No controller found");
  Err(DiscoveryError::NoControllerFound)
}

async fn recv_pack(socket: &UdpSocket) -> Result<DiscoveryResponse, DiscoveryError> {
  let mut buf = [0u8; 1024];
  match socket.recv_from(&mut buf).await {
    Ok((size, addr)) => {
      info!("Received discovery response from {}:{}", addr.ip(), addr.port());
      let msg = str::from_utf8(&buf[..size])?;
      let resp: DiscoveryResponse = DiscoveryResponse::from_str(msg)?;
      if resp.magic == MAGIC_RESPONSE {
        Ok(resp)
      } else {
        error!("Invalid magic: {}", resp.magic);
        Err(DiscoveryError::ProtocolError("Invalid magic"))
      }
    }
    Err(err) => {
      error!("Failed to receive data: {err}");
      Err(err.into())
    }
  }
}

async fn handle_pack(r: Result<DiscoveryResponse, DiscoveryError>) -> Option<Vec<Url>> {
  match r {
    Ok(resp) => match handle_resp(resp).await {
      Ok(wss) => {
        info!("Discovered {} controllers", wss.len());
        if wss.is_empty() {
          warn!("No controllers found");
        } else {
          return Some(wss);
        }
      }
      Err(err) => {
        error!("Failed to handle discovery response: {err}");
      }
    },
    Err(err) => {
      error!("Failed to handle discovery message: {err}");
    }
  }
  None
}

async fn handle_resp(resp: DiscoveryResponse) -> Result<Vec<Url>, DiscoveryError> {
  let ws2: Vec<Url> = join_all(resp.ws.iter().map(async |ws| -> Option<Url> { handle_url(ws.as_str()).await }))
    .await
    .iter()
    .filter_map(|i| i.clone())
    .collect();
  info!("Discovered {} controllers", ws2.len());
  Ok(ws2)
}

async fn https_ping(url: Url) -> bool {
  let r = match reqwest::Client::builder()
    .danger_accept_invalid_certs(true) // Controller may use self-signed certs, and we don't care because it's not being used to authenticate
    .build()
  {
    Ok(v) => v,
    Err(e) => {
      error!("Failed to create HTTP client: {e}");
      return false;
    }
  };
  let r = match r.head(url.clone()).timeout(Duration::from_secs(5)).send().await {
    Ok(v) => v,
    Err(e) => {
      warn!("Failed to send request to {url}: {e}");
      return false;
    }
  };
  r.status().is_success()
}

async fn test_url(url: Url) -> bool {
  let mut url = url.clone();
  match url.scheme() {
    "ws" => {
      if url.set_scheme("http").is_err() {
        error!("Failed to set scheme for {url}");
        return false;
      }
    }
    "wss" => {
      if url.set_scheme("https").is_err() {
        error!("Failed to set scheme for {url}");
        return false;
      }
    }
    "http" | "https" => {}
    _ => {
      error!("Unsupported URL scheme");
      return false;
    }
  }
  https_ping(url).await
}

async fn handle_url(url: &str) -> Option<Url> {
  let Ok(mut url) = Url::from_str(url) else {
    warn!("Invalid URL: {url}");
    return None;
  };
  if !test_url(url.clone()).await {
    warn!("Invalid controller URL: {url}");
    return None;
  }
  match url.scheme() {
    "ws" | "wss" => {}
    "http" => {
      if url.set_scheme("ws").is_err() {
        error!("Failed to set scheme for {url}");
        return None;
      }
    }
    "https" => {
      if url.set_scheme("wss").is_err() {
        error!("Failed to set scheme for {url}");
        return None;
      }
    }
    _ => {
      unreachable!() // This should never happen because we check the scheme in test_url
    }
  }
  Some(url)
}
