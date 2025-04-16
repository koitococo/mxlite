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

use crate::states::SharedAppState;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/by-host", get(get_url_sub_host)).route("/by-ip", get(get_url_sub_ip))
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

async fn get_url_sub_host(State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByHostParams>) -> (StatusCode, Json<GetUrlSubResponse>) {
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

async fn get_url_sub_ip(State(app): State<SharedAppState>, Query(params): Query<GetUrlSubByIpParams>) -> (StatusCode, Json<GetUrlSubResponse>) {
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

fn get_url_sub_ip_inner(params: GetUrlSubByIpParams, port: u16) -> Result<GetUrlSubResponse> {
  let target = ipv4_str_to_u32(&params.ip)?;
  let ips = match_ip(target)?;
  Ok(GetUrlSubResponse {
    ok: true,
    error: None,
    urls: ips.iter().map(|ip| format!("http://{}:{}/{}", ip, port, params.path,)).collect(),
  })
}

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

fn match_ip(target: u32) -> Result<Vec<String>> {
  Ok(
    if_addrs::get_if_addrs()?
      .iter()
      .filter_map(|int| match &int.addr {
        IfAddr::V4(addr) => {
          if addr.prefixlen > 0 && addr.prefixlen < 32 && addr.ip.to_bits() >> (32 - addr.prefixlen) == target >> (32 - addr.prefixlen) {
            Some(addr.ip.to_string())
          } else {
            None
          }
        }
        IfAddr::V6(_) => None, // Not implemented
      })
      .collect(),
  )
}
