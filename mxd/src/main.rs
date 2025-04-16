use anyhow::Result;
use clap::Parser;
use log::{LevelFilter, info};

mod api;
mod discovery;
mod server;
mod srv;
mod states;

#[derive(Parser)]
struct Cli {
  /// Port to listen on
  #[clap(short, long, env = "MXD_PORT", default_value = "8080")]
  port: u16,

  /// API key for authentication
  #[clap(short, long, env = "MXD_APIKEY")]
  apikey: Option<String>,

  /// Path to static files
  #[clap(short, long, env = "MXD_STATIC_PATH")]
  static_path: Option<String>,

  /// Enable discovery
  #[clap(short, long, env = "MXD_DISCOVERY", default_value = "false")]
  disable_discovery: bool,

  /// Enable verbose logging
  #[clap(short, long, default_value = "false")]
  verbose: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct StartupArguments {
  pub(crate) port: u16,
  pub(crate) apikey: Option<String>,
  pub(crate) static_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let config = Cli::parse();

  simple_logger::SimpleLogger::new()
    .with_level(if cfg!(debug_assertions) {
      LevelFilter::Trace
    } else if config.verbose {
      LevelFilter::Debug
    } else {
      LevelFilter::Info
    })
    .with_utc_timestamps()
    .env()
    .init()?;

  info!("MetalX Controller - Launching");

  let discovery_ = if config.disable_discovery {
    None
  } else {
    Some(discovery::serve(config.port))
  };

  if let Err(e) = server::main(StartupArguments {
    port: config.port,
    apikey: config.apikey,
    static_path: config.static_path,
  })
  .await
  {
    log::error!("Failed to start server: {}", e);
  }

  if let Some((join, cancel)) = discovery_ {
    cancel.cancel();
    join.await?;
    log::info!("Discovery server stopped");
  }
  Ok(())
}
