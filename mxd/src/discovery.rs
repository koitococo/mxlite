use std::str::FromStr;

use anyhow::Result;
use common::discovery::{
    DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE, get_multicast_addr,
};
use log::{error, info, trace, warn};
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
                Some(format!("ws://{}:{}/ws", ip, port))
            } else {
                None
            }
        })
        .collect();
    Ok(urls)
}

async fn recv_pack(socket: &UdpSocket, port: u16) -> Result<()> {
    let mut buf = [0u8; 1024];
    trace!("Waiting for discovery request");
    match socket.recv_from(&mut buf).await {
        Ok((size, addr)) => {
            info!(
                "Received discovery request from {}:{}",
                addr.ip(),
                addr.port()
            );
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
            error!("Failed to receive data: {}", err);
            Err(err.into())
        }
    }
}

pub fn serve(port: u16) -> (JoinHandle<()>, CancellationToken) {
    info!("Setting up discovery service");
    let token = CancellationToken::new();
    let token_ = token.clone();
    let join = tokio::spawn(async move {
        if let Ok(socket) = UdpSocket::bind(get_multicast_addr()).await {
            log::info!(
                "Discovery service started at {}",
                socket.local_addr().unwrap()
            );
            loop {
                select! {
                    _ = token_.cancelled() => {
                        info!("Discovery service stopping");
                        break;
                    }
                    r = recv_pack(&socket, port) => {
                        if let Err(err) = r {
                            error!("Failed to handle discovery message: {}", err);
                        }
                    }
                }
            }
        } else {
            error!("Failed to start discovery service");
        }
    });
    (join, token)
}
