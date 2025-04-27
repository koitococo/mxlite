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
  serve::{IncomingStream, Listener},
};
use futures::future::join3;
use log::{debug, error, info};
use serde::Serialize;
use tokio::{
  net::{TcpListener, TcpStream},
  select,
  time::sleep,
};
use tokio_rustls::{
  TlsAcceptor,
  rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
  },
  server::TlsStream,
};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

use crate::{
  StartupArgs,
  states::{AppState, SharedAppState},
};

mod api;
mod collector;
mod net;
mod srv;
mod utils;

struct TlsListener {
  listener: TcpListener,
  acceptor: TlsAcceptor,
}

impl TlsListener {
  async fn try_accept(&mut self) -> Result<(TlsStream<TcpStream>, SocketAddr)> {
    let (io, addr) = self.listener.accept().await?;
    let tls_stream = self.acceptor.accept(io).await?;
    Ok((tls_stream, addr))
  }
}

impl Listener for TlsListener {
  type Addr = SocketAddr;
  type Io = TlsStream<TcpStream>;

  async fn accept(&mut self) -> (Self::Io, Self::Addr) {
    loop {
      match self.try_accept().await {
        Ok(tup) => return tup,
        Err(e) => {
          error!("Error accepting connection: {}", e);
          continue;
        }
      }
    }
  }

  fn local_addr(&self) -> tokio::io::Result<Self::Addr> { self.listener.local_addr() }
}

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
    SocketConnectInfo {
      local_addr,
      remote_addr,
    }
  }
}

impl Connected<IncomingStream<'_, TlsListener>> for SocketConnectInfo {
  fn connect_info(target: IncomingStream<'_, TlsListener>) -> Self {
    let io = target.io().get_ref();
    let local_addr = io.0.local_addr().ok();
    let remote_addr = io.0.peer_addr().ok();
    SocketConnectInfo {
      local_addr,
      remote_addr,
    }
  }
}

pub(crate) async fn main(config: StartupArgs) -> Result<()> {
  let halt_signal = CancellationToken::new();
  let halt_lifecycle = halt_signal.child_token();
  let halt_http = halt_signal.child_token();
  let halt_https = halt_signal.child_token();

  let app: SharedAppState = Arc::new(AppState::new(halt_signal.clone(), config.clone()));
  let mut route = Router::new()
    .route("/ws", get(self::net::handle_ws).head(async || StatusCode::OK))
    .nest("/api", self::api::build(app.clone()))
    .nest("/srv", self::srv::build(app.clone()));
  if let Some(static_path) = config.static_path {
    route = route.nest_service("/static", ServeDir::new(static_path));
  }
  let route_srv = route.with_state(app.clone()).into_make_service_with_connect_info::<SocketConnectInfo>();

  let halt_signal2 = halt_signal.clone();
  let http_port = config.http_port;
  let http_listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), http_port)).await?;
  let http_serve = axum::serve(http_listener, route_srv.clone()).with_graceful_shutdown(async move {
    select! {
        _ = halt_http.cancelled() => {
            info!("Server shutting down");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl-C, shutting down");
            halt_signal2.cancel();
            halt_signal2.cancelled().await;
        }
    }
  });
  info!("HTTP Server started on {}", http_serve.local_addr()?);

  let halt_signal2 = halt_signal.clone();
  let https_serve = if let Some(https) = config.https_args {
    let tls_config = ServerConfig::builder().with_no_client_auth().with_single_cert(
      vec![CertificateDer::from_pem_slice(https.cert.as_bytes())?],
      PrivateKeyDer::from_pem_slice(https.key.as_bytes())?,
    )?;
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), https.port)).await?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let tls_listener = TlsListener { listener, acceptor };
    let serve = axum::serve(tls_listener, route_srv).with_graceful_shutdown(async move {
      select! {
          _ = halt_https.cancelled() => {
              info!("HTTPS Server shutting down");
          }
          _ = tokio::signal::ctrl_c() => {
              info!("Received Ctrl-C, shutting down");
              halt_signal2.cancel();
              halt_signal2.cancelled().await;
          }
      }
    });
    info!("HTTPS Server started on {}", serve.local_addr()?);
    Some(serve)
  } else {
    None
  };

  join3(
    async {
      lifecycle_helper(app.clone(), halt_lifecycle.clone()).await;
    },
    async {
      if let Err(e) = http_serve.await {
        error!("HTTP server error: {}", e);
        halt_signal.cancel();
      }
    },
    async {
      if let Some(s) = https_serve {
        if let Err(e) = s.await {
          error!("HTTPS server error: {}", e);
          halt_signal.cancel();
        }
      }
    },
  )
  .await;
  info!("Server stopped");
  Ok(())
}

async fn lifecycle_helper(_app: SharedAppState, halt_signal: CancellationToken) {
  loop {
    select! {
        _ = halt_signal.cancelled() => {
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
