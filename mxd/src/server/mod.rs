use std::{
  net::{IpAddr, Ipv4Addr, SocketAddr},
  sync::Arc,
  time::Duration,
};

use anyhow::Result;
use axum::{
  Router,
  extract::connect_info::Connected,
  http::StatusCode,
  routing::get,
  serve::IncomingStream,
};
use log::{debug, info};
use serde::Serialize;
use tokio::{net::TcpListener, select, time::sleep};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

use crate::{
  StartupArguments,
  states::{
    AppState, SharedAppState,
  },
};

mod api;
mod collector;
mod net;
mod srv;
mod utils;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct SocketConnectInfo {
  pub(crate) local_addr: Option<SocketAddr>,
  pub(crate) remote_addr: Option<SocketAddr>,
}

impl Connected<IncomingStream<'_, TcpListener>> for SocketConnectInfo {
  fn connect_info(target: IncomingStream<'_, TcpListener>) -> Self {
    let io = target.io();
    let local_addr = io.local_addr().ok();
    let remote_addr = io.peer_addr().ok();
    SocketConnectInfo { local_addr, remote_addr }
  }
}

pub(crate) async fn main(config: StartupArguments) -> Result<()> {
  let halt_signal = CancellationToken::new();
  let halt_signal2 = halt_signal.clone();
  let app: SharedAppState = Arc::new(AppState::new(halt_signal.clone(), config.clone()));
  let mut route = Router::new()
    .route("/ws", get(self::net::handle_ws).head(async || StatusCode::OK))
    .nest("/api", self::api::build(app.clone()))
    .nest("/srv", self::srv::build(app.clone()));
  if let Some(static_path) = config.static_path {
    route = route.nest_service("/static", ServeDir::new(static_path));
  }
  let serve = axum::serve(
    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port)).await?,
    route.with_state(app.clone()).into_make_service_with_connect_info::<SocketConnectInfo>(),
  )
  .with_graceful_shutdown(async move {
    select! {
        _ = halt_signal.cancelled() => {
            info!("Server shutting down");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl-C, shutting down");
            halt_signal.cancel();
            halt_signal.cancelled().await;
        }
    }
  });
  info!("Server started on {}", serve.local_addr()?);

  tokio::spawn(lifetime_helper(app.clone(), halt_signal2.clone()));
  serve.await?;
  info!("Server stopping");
  halt_signal2.cancel();
  halt_signal2.cancelled().await;
  info!("Server stopped");
  Ok(())
}

async fn lifetime_helper(_app: SharedAppState, halt_signal: CancellationToken) {
  let cancellation_token = halt_signal.child_token(); // to avoid unused variable warning, maybe used in the future
  loop {
    select! {
        _ = cancellation_token.cancelled() => {
            debug!("lifetime helper is shutting down");
            break;
        }
        _ = sleep(Duration::from_secs(15)) => {
            // trace!("Performing periodic tasks");
            // helper_heartbeat(app.clone()).await;
        }
    }
  }
}

// async fn helper_heartbeat(app: SharedAppState) {
//     let _ = app.host_session.list_sessions().await;
// }
