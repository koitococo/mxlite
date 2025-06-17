use clap::Parser;
use std::fs;

use self::utils::random_str;
use anyhow::Result;
use log::{error, info, warn};

pub mod executor;
pub mod net;
pub mod script;
pub mod utils;

#[derive(Parser, Debug)]
#[command(version = crate::VERSION)]
struct Cli {
  /// Connect to controller with Websocket URL. This option will disable discovery.
  #[clap(short, long, env = "MXA_WS_URL")]
  ws_url: Option<String>,

  /// Be verbose
  #[clap(short, long, env = "MXA_VERBOSE")]
  verbose: bool,

  /// Execute provided lua script. This option will not start agent.
  #[clap(long)]
  script: Option<String>,
}

pub async fn main() -> Result<()> {
  let cli = Cli::parse();

  crate::logger::install_logger(cli.verbose);

  if let Some(script_path) = cli.script {
    return script_main(script_path).await;
  }

  info!("MetalX Agent - Launching");

  #[cfg(unix)]
  {
    if !nix::unistd::geteuid().is_root() {
      warn!("Running mxa as unprivileged user may cause permission issues");
    }
  }

  let host_id = utils::get_machine_id().unwrap_or_else(|| {
    error!("Failed to get machine id");
    if cfg!(debug_assertions) {
      "cafecafe-cafe-cafe-cafe-cafecafecafe".to_string()
    } else {
      utils::get_random_uuid()
    }
  });

  let session_id = random_str(16);
  info!("Host ID: {host_id}");
  info!("Session ID: {session_id}");
  let envs = std::env::vars()
    .filter(|(k, _)| k.starts_with("MX_"))
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>();

  loop {
    match net::handle_ws_url(cli.ws_url.clone(), host_id.clone(), session_id.clone(), envs.clone()).await {
      Err(err) => {
        error!("Agent failed: {err}");
      }
      Ok(exit) => {
        if exit {
          return Ok(());
        }
      }
    }
  }
}

async fn script_main(script: String) -> Result<()> {
  let content = match fs::read_to_string(script) {
    Ok(content) => content,
    Err(e) => {
      error!("Failed to read script: {e}");
      return Ok(());
    }
  };
  let ctx = script::ExecutorContext::try_new()?;
  if let Err(e) = ctx.exec_async(&content).await {
    error!("Failed to execute script: {e}");
  } else {
    info!("Script executed successfully");
  }
  Ok(())
}
