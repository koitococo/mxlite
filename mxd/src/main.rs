use anyhow::Result;
use clap::Parser;
use log::{LevelFilter, info};

mod api;
mod discovery;
mod file_service;
mod server;
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
    #[clap(short, long, env = "MXD_STATIC_PATH", default_value = "./static")]
    static_path: String,

    /// Enable verbose logging
    #[clap(short, long, default_value = "false")]
    verbose: bool,
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

    let (join, cancel) = discovery::serve(config.port);
    if let Err(e) = server::main(config).await {
        log::error!("Failed to start server: {}", e);
    }
    cancel.cancel();
    join.await?;
    Ok(())
}
