use std::vec;

use anyhow::Result;
use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  routing::get,
};
use if_addrs::IfAddr;
use log::error;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::states::{SharedAppState, host_session::ExtraInfo};

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new()
    .with_state(app.clone())
    .route("/by-host", get(get_url_sub_host))
    .route("/by-host-ip", get(get_url_sub_host_ip))
    .route("/by-ip", get(get_url_sub_ip))
    .route("/remote-by-host-ip", get(get_remote_url_sub_host_ip))
    
}

#[derive(Deserialize)]
struct GetUrlSubByHostParams {
  host: String,
  path: String,
}

#[derive(Deserialize)]
struct GetUrlSubByIpParams {
  ip: String,
  path: String,
}

#[derive(Serialize)]
struct GetUrlSubResponse {
  ok: bool,
  error: Option<String>,
  urls: Vec<String>,
}

async fn get_url_sub_host(
  State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByHostParams>,
) -> (StatusCode, Json<GetUrlSubResponse>) {
  if let Some(info) = app.host_session.get(&params.host).map(|s| s.extra.clone()) {
    if let Ok(mut url) = Url::parse(&info.controller_url) {
      if url.set_scheme("http").is_err() {
        return (
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(GetUrlSubResponse {
            ok: false,
            error: Some("Invalid URL scheme".to_string()),
            urls: vec![],
          }),
        );
      }
      url.set_path(&params.path);
      (
        StatusCode::OK,
        Json(GetUrlSubResponse {
          ok: true,
          error: None,
          urls: vec![url.to_string()],
        }),
      )
    } else {
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetUrlSubResponse {
          ok: false,
          error: Some("Host provided bad info".to_string()),
          urls: vec![],
        }),
      )
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(GetUrlSubResponse {
        ok: false,
        error: Some("Host not found".to_string()),
        urls: vec![],
      }),
    )
  }
}

async fn get_url_sub_host_ip(
  State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByHostParams>,
) -> (StatusCode, Json<GetUrlSubResponse>) {
  let port = app.startup_args.port;
  if let Some(info) = app.host_session.get(&params.host).map(|s| s.extra.clone()) {
    if let Ok(local_nets) = get_local_ips() {
      let remote_nets = get_remote_ips(info);
      let matches = find_all_routable(remote_nets, local_nets.iter().map(|local_ip| local_ip.0));
      let urls = matches.iter().map(|ip| format!("http://{}:{}/{}", u32_to_ipv4_str(*ip), port, params.path,).to_string()).collect();
      (
        StatusCode::OK,
        Json(GetUrlSubResponse {
          ok: true,
          error: None,
          urls,
        }),
      )
    } else {
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetUrlSubResponse {
          ok: false,
          error: Some("Failed to get local IPs".to_string()),
          urls: vec![],
        }),
      )
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(GetUrlSubResponse {
        ok: false,
        error: Some("Host not found".to_string()),
        urls: vec![],
      }),
    )
  }
}

async fn get_url_sub_ip(
  State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByIpParams>,
) -> (StatusCode, Json<GetUrlSubResponse>) {
  match get_url_sub_ip_inner(params, app.startup_args.port) {
    Ok(resp) => (StatusCode::OK, Json(resp)),
    Err(err) => {
      error!("Error: {}", err);
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetUrlSubResponse {
          ok: false,
          error: Some(err.to_string()),
          urls: vec![],
        }),
      )
    }
  }
}

async fn get_remote_url_sub_host_ip(
  State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByHostParams>,
) -> (StatusCode, Json<GetUrlSubResponse>) {
  let port = app.startup_args.port;
  if let Some(info) = app.host_session.get(&params.host).map(|s| s.extra.clone()) {
    if let Ok(local_nets) = get_local_ips() {
      let remote_nets = get_remote_ips(info);
      let matches = find_all_routable(local_nets, remote_nets.iter().map(|local_ip| local_ip.0));
      let urls = matches.iter().map(|ip| format!("http://{}:{}/{}", u32_to_ipv4_str(*ip), port, params.path,).to_string()).collect();
      (
        StatusCode::OK,
        Json(GetUrlSubResponse {
          ok: true,
          error: None,
          urls,
        }),
      )
    } else {
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(GetUrlSubResponse {
          ok: false,
          error: Some("Failed to get local IPs".to_string()),
          urls: vec![],
        }),
      )
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(GetUrlSubResponse {
        ok: false,
        error: Some("Host not found".to_string()),
        urls: vec![],
      }),
    )
  }
}

#[inline]
fn get_url_sub_ip_inner(params: GetUrlSubByIpParams, port: u16) -> Result<GetUrlSubResponse> {
  let target = ipv4_str_to_u32(&params.ip)?;
  let ips = match_local_ip(target)?;
  Ok(GetUrlSubResponse {
    ok: true,
    error: None,
    urls: ips.iter().map(|ip| format!("http://{}:{}/{}", ip, port, params.path,)).collect(),
  })
}

#[inline]
fn ipv4_str_to_u32(ip: &str) -> Result<u32> {
  let parts: Vec<&str> = ip.split('.').collect();
  if parts.len() != 4 {
    return Err(anyhow::anyhow!("Invalid IP address format"));
  }
  let mut result = 0u32;
  for part in parts {
    let octet: u32 = part.parse()?;
    result = (result << 8) | octet;
  }
  Ok(result)
}

#[inline]
fn u32_to_ipv4_str(ip: u32) -> String {
  format!(
    "{}.{}.{}.{}",
    (ip >> 24) & 0xFF,
    (ip >> 16) & 0xFF,
    (ip >> 8) & 0xFF,
    ip & 0xFF
  )
}

#[inline]
fn get_local_ips() -> Result<Vec<(u32, u8)>> {
  let ifaddrs = if_addrs::get_if_addrs()?;
  let nets = ifaddrs
    .iter()
    .filter_map(|int| match &int.addr {
      IfAddr::V4(addr) => {
        let ip = addr.ip.to_bits();
        if is_in_subnet(ip, 0x7f00_0000, 8) {
          None
        } else {
          Some((ip, addr.prefixlen))
        }
      }
      IfAddr::V6(_) => None, // Not implemented
    })
    .collect::<Vec<(u32, u8)>>();
  Ok(nets)
}

#[inline]
fn get_remote_ips(info: ExtraInfo) -> Vec<(u32, u8)> {
  info
    .system_info
    .nics
    .iter()
    .map(|nic| {
      nic.ip.iter().filter_map(|ip| {
        if ip.version == 4 {
          if let Ok(ipv4) = ipv4_str_to_u32(ip.addr.as_str()) {
            return Some((ipv4, ip.prefix));
          }
        }
        None
      })
    })
    .fold(Vec::new(), |mut s: Vec<(u32, u8)>, i| {
      let nic_ip = i.collect::<Vec<(u32, u8)>>();
      s.extend(nic_ip);
      s
    })
}

#[inline]
fn match_local_ip(target: u32) -> Result<Vec<String>> {
  let nets = get_local_ips()?;
  let results = nets.iter().filter_map(|(ip, prefixlen)| {
    if is_in_subnet(target, *ip, *prefixlen) {
      Some(u32_to_ipv4_str(*ip))
    } else {
      None
    }
  }).collect::<Vec<String>>();
  Ok(results)
}

#[inline]
fn is_in_subnet(ip: u32, net: u32, prefixlen: u8) -> bool {
  prefixlen > 0 && prefixlen < 32 && ip >> (32 - prefixlen) == net >> (32 - prefixlen)
}

#[inline]
fn find_all_routable<N: IntoIterator<Item = (u32, u8)>, T: IntoIterator<Item = u32>>(nets: N, targets: T) -> Vec<u32> {
  let mut targets = targets.into_iter().collect::<Vec<u32>>();
  let mut results: Vec<u32> = Vec::with_capacity(targets.len() / 2);
  for net in nets {
    let (net, prefixlen) = net;
    targets.retain(|&target| {
      if is_in_subnet(target, net, prefixlen) {
        results.push(target);
        false
      } else {
        true
      }
    });
  }
  results
}
