use anyhow::{Result, anyhow};
use common::protocol::discovery::{
    DISCOVERY_PORT, DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE,
    PROTOCOL_REV,
};
use futures_util::future::join_all;
use log::{debug, error, info, trace, warn};
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::{self, FromStr};
use std::time::Duration;
use tokio::{net::UdpSocket, select};

pub async fn discover_controller() -> Result<Vec<String>> {
    loop {
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
            trace!("Sending discovery request: {}", req_str);
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
    }
}

async fn recv_pack(socket: &UdpSocket) -> Result<DiscoveryResponse> {
    let mut buf = [0u8; 1024];
    match socket.recv_from(&mut buf).await {
        Ok((size, addr)) => {
            info!(
                "Received discovery response from {}:{}",
                addr.ip(),
                addr.port()
            );
            let msg = str::from_utf8(&buf[..size])?;
            let resp: DiscoveryResponse = DiscoveryResponse::from_str(msg)?;
            if resp.magic == MAGIC_RESPONSE {
                Ok(resp)
            } else {
                error!("Invalid magic: {}", resp.magic);
                Err(anyhow!("Invalid magic"))
            }
        }
        Err(err) => {
            error!("Failed to receive data: {}", err);
            Err(err.into())
        }
    }
}

async fn handle_pack(r: Result<DiscoveryResponse>) -> Option<Vec<String>> {
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
                error!("Failed to handle discovery response: {}", err);
            }
        },
        Err(err) => {
            error!("Failed to handle discovery message: {}", err);
        }
    }
    None
}

async fn handle_resp(resp: DiscoveryResponse) -> Result<Vec<String>> {
    // let mut wss = Vec::new();
    // for ws in resp.ws {
    //     let mut url = Url::from_str(ws.as_str())?;
    //     if url.set_scheme("http").is_err() {
    //         warn!("Invalid URL: {}", ws);
    //         continue;
    //     }
    //     debug!("Pinging controller with url: {}", ws);
    //     if let Err(e) = http_ping(url, 5).await {
    //         warn!("Failed to ping controller: {}: {}", ws, e);
    //         continue;
    //     }
    //     info!("Discovered controller: {}", ws);
    //     wss.push(ws);
    // }
    let ws2: Vec<String> = join_all(resp.ws.iter().map(async |ws: &String| -> Option<String> {
        if let Ok(mut url) = Url::from_str(ws.as_str()) {
            if url.set_scheme("http").is_err() {
                warn!("Invalid URL: {}", ws);
                return None;
            }
            debug!("Pinging controller with url: {}", ws);
            if let Err(e) = http_ping(url, 5).await {
                warn!("Failed to ping controller: {}: {}", ws, e);
                return None;
            }
            info!("Discovered controller: {}", ws);
            return Some(ws.clone());
        }
        None
    }))
    .await
    .iter()
    .filter_map(|i| i.clone())
    .collect();
    info!("Discovered {} controllers", ws2.len());
    Ok(ws2)
}

async fn http_ping(url: Url, timeout: u64) -> Result<bool> {
    Ok(reqwest::Client::new()
        .head(url)
        .timeout(Duration::from_secs(timeout))
        .send()
        .await?
        .status()
        .is_success())
}
