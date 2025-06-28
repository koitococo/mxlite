use std::{
  net::{IpAddr, Ipv4Addr, SocketAddr},
  str::FromStr,
};

use crate::{
  daemon::states::AppState,
  protocol::discovery::{DISCOVERY_PORT, DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE},
};
use anyhow::Result;
use log::{debug, error, info, warn};
use tokio::{net::UdpSocket, select, task::JoinHandle};
use tokio_util::sync::CancellationToken;

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

async fn recv_pack(socket: &UdpSocket, http_port: u16) -> Result<()> {
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
          ws: get_ws_urls(http_port)?,
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

async fn discovery_main(ct: CancellationToken, http_port: u16) {
  match UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT)).await {
    Ok(socket) => {
      log::info!("Discovery service started at {}", socket.local_addr().unwrap());
      loop {
        select! {
            _ = ct.cancelled() => {
                info!("Discovery service stopping");
                break;
            }
            r = recv_pack(&socket, http_port) => {
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
}

pub struct DiscoveryService {
  http_port: u16,
  join_handle: Option<JoinHandle<()>>,
  main_ct: CancellationToken,
  sub_ct: Option<CancellationToken>,
  started: bool,
}

impl DiscoveryService {
  pub fn new(state: &AppState) -> Option<Self> {
    if state.startup_args.disable_discovery {
      info!("Discovery service is disabled");
      return None;
    }

    Some(DiscoveryService {
      http_port: state.startup_args.http_port,
      join_handle: None,
      main_ct: state.cancel_signal.clone(),
      sub_ct: None,
      started: false,
    })
  }

  pub fn start(&mut self) -> Result<()> {
    if self.started {
      info!("Discovery service is already running");
      return Ok(());
    }
    info!("Setting up discovery service");
    let ct = self.main_ct.child_token();
    let ct_clone = ct.clone();
    let http_port = self.http_port;
    let join = tokio::spawn(async move {
      discovery_main(ct_clone, http_port).await;
    });
    self.join_handle = Some(join);
    self.sub_ct = Some(ct);
    self.started = true;
    Ok(())
  }

  pub async fn stop(&mut self) -> Result<()> {
    if !self.started {
      info!("Discovery service is not running");
      return Ok(());
    }
    info!("Stopping discovery service");
    if let Some(ct) = self.sub_ct.take() {
      ct.cancel();
    } else {
      error!("Discovery service was not running");
      anyhow::bail!("Discovery service was not running");
    }

    if let Some(join_handle) = self.join_handle.take() {
      match join_handle.await {
        Ok(_) => info!("Discovery service stopped successfully"),
        Err(e) => error!("Failed to stop discovery service: {e}"),
      }
    } else {
      error!("Discovery service was not running");
      anyhow::bail!("Discovery service was not running");
    }
    self.started = false;
    Ok(())
  }

  pub fn running(&self) -> bool {
    self.started
  }
}
