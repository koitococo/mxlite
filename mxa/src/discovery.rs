use anyhow::{Result, anyhow};
use common::discovery::{
    DiscoveryRequest, DiscoveryResponse, MAGIC_REQUEST, MAGIC_RESPONSE, PROTOCOL_REV,
    get_multicast_addr,
};
use log::{error, info, warn};
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;
use tokio::{net::UdpSocket, select};

async fn recv_pack(socket: &UdpSocket) -> Result<Vec<String>> {
    let mut buf = [0u8; 1024];
    match socket.recv_from(&mut buf).await {
        Ok((size, addr)) => {
            info!(
                "Received discovery response from {}:{}",
                addr.ip(),
                addr.port()
            );
            let msg = std::str::from_utf8(&buf[..size])?;
            let resp: DiscoveryResponse = DiscoveryResponse::from_str(msg)?;
            if resp.magic == MAGIC_RESPONSE {
                let mut wss = Vec::new();
                let client = reqwest::Client::new();
                for ws in resp.ws {
                    let mut url = Url::from_str(ws.as_str())?;
                    if url.set_scheme("http").is_err() {
                        warn!("Invalid URL: {}", ws);
                        continue;
                    }
                    let http_ping = client.head(url).send().await?;
                    if !http_ping.status().is_success() {
                        warn!("Failed to ping controller: {}", ws);
                        continue;
                    }
                    info!("Discovered controller: {}", ws);
                    wss.push(ws);
                }
                Ok(wss)
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

pub async fn discover_controller() -> Result<Vec<String>> {
    info!("Discovering controller via multicast");
    // Bind to any available port
    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)).await?;

    let req = &DiscoveryRequest {
        magic: MAGIC_REQUEST.to_string(),
        addr: socket.local_addr()?,
        revision: PROTOCOL_REV,
    };
    let req_str = req.to_string();
    let req_bin = req_str.as_bytes();
    socket.send_to(req_bin, get_multicast_addr()).await?;

    let mut responses = Vec::new();

    loop {
        select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                info!("Discovery timeout");
                break;
            }
            r = recv_pack(&socket) => {
                if let Ok(r) = r {
                    responses.extend(r);
                }
            }
        }
    }
    Ok(responses)
}
