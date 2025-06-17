use anyhow::Result;
use clap::Parser;
use utils::get_cert_from_file;
use log::{info, warn};

pub mod discovery;
pub mod script;
pub mod server;
pub mod states;
pub mod utils;

#[derive(Parser, Debug)]
#[command(version = crate::VERSION)]
struct Cli {
  /// HTTP port to listen on
  #[clap(short = 'p', long, env = "MXD_PORT", default_value = "8080")]
  http_port: u16,

  /// HTTPS port to listen on
  #[clap(short = 'P', long, env = "MXD_HTTPS_PORT", default_value = "8443")]
  https_port: u16,

  /// API key for authentication, optional.
  /// If not provided, authentication will be disabled.
  #[clap(short = 'k', long, env = "MXD_APIKEY")]
  apikey: Option<String>,

  /// Path to static files
  #[clap(short = 's', long, env = "MXD_STATIC_PATH")]
  static_path: Option<String>,

  /// Disable agent discovery
  #[clap(short = 'd', long, env = "MXD_DISCOVERY", default_value = "false")]
  disable_discovery: bool,

  /// Enable verbose logging
  #[clap(short = 'v', long, env = "MXD_VERBOSE", default_value = "false")]
  verbose: bool,

  /// Detect other controllers by broadcasting on the network
  #[clap(short = 'D', long, env = "MXD_DETECT_OTHERS", default_value = "false")]
  detect_others: bool,

  /// Enable http service
  ///
  /// Disable http service will also make discovery service and https services disabled
  #[clap(short = 'T', long, env = "MXD_HTTP", default_value = "true")]
  http: bool,

  /// Enable https service. Requires TLS certificate and key.
  ///
  /// To use existed TLS certificate:
  ///   --https --tls-cert <existed_file> --tls-key <existed_file>
  ///
  /// To generate TLS certificate with existed CA:
  ///   --https --generate-cert --tls-cert <non-existed_file> --tls-key <non-existed_file> --ca-cert <existed_file> --ca-key <existed_file>
  ///
  /// To generate both CA and TLS cert:
  ///   --https --generate-cert --tls-cert <non-existed_file> --tls-key <non-existed_file> --ca-cert <non-existed_file> --ca-key <non-existed_file>
  #[clap(short = 't', long, env = "MXD_HTTPS", default_value = "false")]
  https: bool,

  /// TLS certificate file
  #[clap(short = 'c', long, env = "MXD_TLS_CERT")]
  tls_cert: Option<String>,

  /// TLS key file
  #[clap(short = 'e', long, env = "MXD_TLS_KEY")]
  tls_key: Option<String>,

  /// Path to the generated CA certificate
  #[clap(short = 'C', long, env = "MXD_CA_CERT")]
  ca_cert: Option<String>,

  /// Path to the generated CA key
  #[clap(short = 'E', long, env = "MXD_CA_KEY")]
  ca_key: Option<String>,

  /// Generate self-signed certificate on startup with ECDSA signing using the P-256 curves and SHA-256 hashing
  ///
  /// The generated certificate will be valid for 7 days and generated CA for 30 days.
  ///
  /// Must be used with `--https`, `--tls-cert`,`--tls-key`, `--ca-cert`, `--ca-key`.
  #[clap(short = 'g', long, env = "MXD_GENERATE_CERT", default_value = "false")]
  generate_cert: bool,

  /// Execute provided lua script. This option will not start server.
  #[clap(long)]
  script: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HttpsArgs {
  pub cert: String,
  pub key: String,
  pub port: u16,
}

#[derive(Clone, Debug)]
pub struct StartupArgs {
  pub enable_http: bool,
  pub http_port: u16,
  pub https_args: Option<HttpsArgs>,
  pub apikey: Option<String>,
  pub static_path: Option<String>,
  pub disable_discovery: bool,
  pub detect_others: bool,
}

impl TryFrom<Cli> for StartupArgs {
  type Error = anyhow::Error;

  fn try_from(config: Cli) -> Result<Self, Self::Error> {
    let args = StartupArgs {
      enable_http: config.http,
      http_port: config.http_port,
      https_args: if config.https {
        let (cert, key) = get_cert_from_file(
          config.tls_cert,
          config.tls_key,
          config.ca_cert,
          config.ca_key,
          config.generate_cert,
        )?;
        Some(HttpsArgs {
          cert,
          key,
          port: config.https_port,
        })
      } else {
        None
      },
      apikey: config.apikey,
      static_path: config.static_path,
      disable_discovery: config.disable_discovery,
      detect_others: config.detect_others,
    };
    Ok(args)
  }
}

pub async fn main() -> Result<()> {
  let cli = Cli::parse();

  if let Some(script) = &cli.script {
    info!("Executing script: {script}");
    let ctx = script::ExecutorContext::try_new()?;
    ctx.exec_async(script).await?;
    info!("Script executed successfully");
    return Ok(());
  }

  crate::logger::install_logger(cli.verbose);

  let args = StartupArgs::try_from(cli)?;
  info!("MetalX Controller - Launching");

  if args.detect_others {
    info!("Detecting other controllers...");
    match crate::discovery::discover_controller_once().await {
      Err(crate::discovery::DiscoveryError::NoControllerFound) => {
        info!("No other controller found");
      }
      Ok(controllers) => {
        warn!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        warn!("Discovered {} controllers", controllers.len());
        for controller in controllers {
          warn!("Controller: {controller}");
        }
        warn!("Please check if you are running multiple controllers on the same network");
        warn!("This may cause conflicts and unexpected behavior");
        warn!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
      }
      Err(err) => {
        log::error!("Failed to discover other controllers: {err}");
      }
    }
  }

  let discovery_ = discovery::serve(args.clone());

  if !args.enable_http {
    info!("HTTP server is disabled");
    return Ok(());
  } else if let Err(e) = server::main(args).await {
    log::error!("Failed to start server: {e}");
  }

  if let Some((join, cancel)) = discovery_ {
    cancel.cancel();
    join.await?;
    log::info!("Discovery server stopped");
  }
  Ok(())
}
