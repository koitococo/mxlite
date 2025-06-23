use log::info;
use tokio::{select, signal};

/// Wait for a Ctrl-C signal(SIGINT) or SIGTERM to gracefully shut down the application.
pub async fn ctrl_c() {
  #[cfg(unix)]
  {
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    select! {
      _ = signal::ctrl_c() => {
        info!("Received Ctrl-C, shutting down");
      }
      _ = sigterm.recv() => {
        info!("Received SIGTERM, shutting down");
      }
    };
  }
  #[cfg(not(unix))]
  {
    let _ = signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down");
  }
}
