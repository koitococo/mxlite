use anyhow::Result;
use clap::Parser;
use log::{error, info};
use utils::random_str;

mod executor;
mod net;
mod utils;

#[derive(Parser, Debug)]
struct Cli {
  #[clap(short, long, env = "MXA_WS_URL")]
  ws_url: Option<String>,

  #[clap(short, long, env = "MXA_VERBOSE")]
  verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
  let config = Cli::parse();

  common::logger::install_logger(config.verbose);

  info!("MetalX Agent - Launching");
  let host_id = match utils::get_machine_id() {
    Some(id) => id,
    None => {
      error!("Failed to get machine id");
      if cfg!(debug_assertions) {
        "cafecafe-cafe-cafe-cafe-cafecafecafe".to_string()
      } else {
        utils::get_random_uuid()
      }
    }
  };
  let session_id = random_str(16);
  info!("Host ID: {}", host_id);
  info!("Session ID: {}", session_id);
  let envs = std::env::vars()
    .filter(|(k, _)| k.starts_with("MX_"))
    .map(|(k, v)| format!("{}={}", k, v))
    .collect::<Vec<_>>();

  loop {
    match net::handle_ws_url(config.ws_url.clone(), host_id.clone(), session_id.clone(), envs.clone()).await {
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
