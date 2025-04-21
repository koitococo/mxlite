use anyhow::Result;
use clap::Parser;
use log::{LevelFilter, info, warn};
use utils::get_cert_from_file;

mod discovery;
mod server;
mod states;
mod utils;

#[derive(Parser)]
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
}

#[derive(Clone, Debug)]
pub(crate) struct HttpsArgs {
  pub(crate) cert: String,
  pub(crate) key: String,
  pub(crate) port: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct StartupArgs {
  pub(crate) enable_http: bool,
  pub(crate) http_port: u16,
  pub(crate) https_args: Option<HttpsArgs>,
  pub(crate) apikey: Option<String>,
  pub(crate) static_path: Option<String>,
  pub(crate) disable_discovery: bool,
  pub(crate) detect_others: bool,
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
    .with_local_timestamps()
    .env()
    .init()?;

  let args = StartupArgs::try_from(config)?;
  info!("MetalX Controller - Launching");

  if args.detect_others {
    info!("Detecting other controllers...");
    match common::discovery::discover_controller_once().await {
      Err(common::discovery::DiscoveryError::NoControllerFound) => {
        info!("No other controller found");
      }
      Ok(controllers) => {
        warn!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        warn!("Discovered {} controllers", controllers.len());
        for controller in controllers {
          warn!("Controller: {}", controller);
        }
        warn!("Please check if you are running multiple controllers on the same network");
        warn!("This may cause conflicts and unexpected behavior");
        warn!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
      }
      Err(err) => {
        log::error!("Failed to discover other controllers: {}", err);
      }
    }
  }

  let discovery_ = discovery::serve(args.clone());

  if !args.enable_http {
    info!("HTTP server is disabled");
    return Ok(());
  } else if let Err(e) = server::main(args).await {
    log::error!("Failed to start server: {}", e);
  }

  if let Some((join, cancel)) = discovery_ {
    cancel.cancel();
    join.await?;
    log::info!("Discovery server stopped");
  }
  Ok(())
}
