use anyhow::Result;
use log::{LevelFilter, error, info};
use utils::random_str;

mod executor;
mod net;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
  simple_logger::SimpleLogger::new()
    .with_level(if cfg!(debug_assertions) { LevelFilter::Trace } else { LevelFilter::Info })
    .with_utc_timestamps()
    .env()
    .init()?;

  info!("MetalX Agent - Launching");
  let host_id = match utils::get_machine_id() {
    Ok(id) => id,
    Err(err) => {
      error!("Failed to get machine id: {}", err);
      if cfg!(debug_assertions) {
        "cafecafecafecafecafecafecafecafe".to_string()
      } else {
        std::process::exit(1);
      }
    }
  };
  let session_id = random_str(16);
  info!("Host ID: {}", host_id);
  info!("Session ID: {}", session_id);
  let envs = std::env::vars().filter(|(k, _)| k.starts_with("MXA_")).map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>();

  loop {
    let ws_url = std::env::var("MXD_URL").ok();
    match net::handle_ws_url(ws_url.clone(), host_id.clone(), session_id.clone(), envs.clone()).await {
      Err(err) => {
        error!("Agent failed: {}", err);
      }
      Ok(exit) => {
        if exit {
          return Ok(());
        }
      }
    }
  }
}
