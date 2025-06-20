use clap::Parser;
use std::fs;

use crate::utils::util::{get_random_uuid, random_str};
use anyhow::Result;
use log::{error, info, warn};

#[derive(Parser, Debug)]
#[command(version = crate::VERSION)]
struct Cli {
  /// Connect to controller with Websocket URL. This option will disable discovery.
  /// 
  /// Url should be in the format of `ws://<host>:<port>` or `wss://<host>:<port>`.
  #[clap(short, long, env = "MXA_WS_URL")]
  ws_url: Option<String>,

  /// Be verbose
  #[clap(short, long, env = "MXA_VERBOSE")]
  verbose: bool,

  /// Execute provided lua script. This option will not start agent.
  #[clap(long)]
  script: Option<String>,

  /// Public key for agent identity authentication.
  #[clap(long, env = "MXA_PUBLIC_KEY")]
  public_key: Option<String>,

  /// Private key for agent identity authentication.
  #[clap(long, env = "MXA_PRIVATE_KEY")]
  private_key: Option<String>,

  /// Enforce authentication of remote controller.
  /// 
  /// If set to true, agent will only connect to controllers that are in the trusted controllers list. 
  /// 
  /// If set to false, agent will connect to any controller, but show a warning if the controller is not in the trusted controllers list.
  #[clap(long, env = "MXA_ENFORCE_AUTH")]
  enforce_auth: bool,

  /// List of trusted controllers.
  /// Each controller should be sha256 hash of controller's public key.
  #[clap(long, env = "MXA_TRUSTED_CONTROLLERS")]
  trusted_controllers: Vec<String>
}

#[derive(Debug, Clone)]
pub(crate) struct StartupArgs {
  pub env_ws_url: Option<String>,
  pub host_id: String,
  pub session_id: String,
  pub envs: Vec<String>,
  pub public_key: Option<String>,
  pub private_key: Option<String>,
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

  let host_id = super::utils::get_machine_id().unwrap_or_else(|| {
    error!("Failed to get machine id");
    if cfg!(debug_assertions) {
      "cafecafe-cafe-cafe-cafe-cafecafecafe".to_string()
    } else {
      get_random_uuid()
    }
  });

  let session_id = random_str(16);
  info!("Host ID: {host_id}");
  info!("Session ID: {session_id}");
  let envs = std::env::vars()
    .filter(|(k, _)| k.starts_with("MX_"))
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>();

  let startup_args = StartupArgs {
    env_ws_url: cli.ws_url.clone(),
    host_id: host_id.clone(),
    session_id: session_id.clone(),
    envs: envs.clone(),
    public_key: cli.public_key.clone(),
    private_key: cli.private_key.clone(),
  };

  loop {
    match super::net::handle_ws_url(startup_args.clone()).await {
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
  let ctx = super::script::ExecutorContext::try_new()?;
  if let Err(e) = ctx.exec_async(&content).await {
    error!("Failed to execute script: {e}");
  } else {
    info!("Script executed successfully");
  }
  Ok(())
}
