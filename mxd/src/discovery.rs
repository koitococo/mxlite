use std::{
  net::{IpAddr, Ipv4Addr, SocketAddr},
  str::FromStr,
};

use anyhow::Result;
use common::protocol::discovery::{DISCOVERY_PORT, DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE};
use log::{debug, error, info, warn};
use tokio::{net::UdpSocket, select, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::StartupArgs;

fn get_ws_urls(port: u16) -> Result<Vec<String>> {
  let urls = if_addrs::get_if_addrs()?
    .iter()
    .filter_map(|if_| {
      if if_.is_loopback() {
        return None;
      }
      let ip = if_.ip();
      if ip.is_ipv4() {
        Some(format!("ws://{ip}:{port}/ws"))
      } else {
        None
      }
    })
    .collect();
  Ok(urls)
}

async fn recv_pack(socket: &UdpSocket, port: u16) -> Result<()> {
  let mut buf = [0u8; 1024];
  debug!("Waiting for discovery request");
  match socket.recv_from(&mut buf).await {
    Ok((size, addr)) => {
      info!("Received discovery request from {}:{}", addr.ip(), addr.port());
      let msg = std::str::from_utf8(&buf[..size])?;
      let req = DiscoveryRequest::from_str(msg)?;
      if req.magic == MAGIC_REQUEST {
        let resp = DiscoveryResponse {
          magic: MAGIC_RESPONSE.to_string(),
          ws: get_ws_urls(port)?,
        };
        let resp_str = resp.to_string();
        socket.send_to(resp_str.as_bytes(), addr).await?;
        info!("Sent discovery response to {}:{}", addr.ip(), addr.port());
      } else {
        warn!("Invalid magic: {}", req.magic);
      }
      Ok(())
    }
    Err(err) => {
      error!("Failed to receive data: {err}");
      Err(err.into())
    }
  }
}

pub fn serve(args: StartupArgs) -> Option<(JoinHandle<()>, CancellationToken)> {
  if !args.enable_http {
    info!("HTTP server is disabled");
    return None;
  }
  if args.disable_discovery {
    info!("Discovery service is disabled");
    return None;
  }
  let port = args.http_port;
  info!("Setting up discovery service");
  let token = CancellationToken::new();
  let token_ = token.clone();
  let join = tokio::spawn(async move {
    match UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT)).await {
      Ok(socket) => {
        log::info!("Discovery service started at {}", socket.local_addr().unwrap());
        loop {
          select! {
              _ = token_.cancelled() => {
                  info!("Discovery service stopping");
                  break;
              }
              r = recv_pack(&socket, port) => {
                  if let Err(err) = r {
                      error!("Failed to handle discovery message: {err}");
                  }
              }
          }
        }
      }
      Err(e) => {
        error!("Failed to start discovery service: {e:?}");
      }
    }
  });
  Some((join, token))
}
