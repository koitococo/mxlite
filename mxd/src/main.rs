use std::env;

use anyhow::Result;
use log::{LevelFilter, info};

mod api;
mod discovery;
mod file_service;
mod server;
mod states;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(if cfg!(debug_assertions) {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        })
        .with_utc_timestamps()
        .env()
        .init()?;

    info!("MetalX Controller - Launching");
    let port = env::var("MXD_PORT")
        .unwrap_or("8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    let apikey = env::var("MXD_APIKEY").ok();

    let (join, cancel) = discovery::serve(port);
    if let Err(e) = server::main(apikey, port).await {
        log::error!("Failed to start server: {}", e);
    }
    cancel.cancel();
    join.await?;
    Ok(())
}
