use std::env;

use anyhow::Result;
use log::info;

mod api;
mod discovery;
mod server;
mod states;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .env()
        .with_utc_timestamps()
        .init()
        .unwrap();

    info!("MetalX Controller - Launching");
    let apikey = env::var("MXD_APIKEY").unwrap_or("api_key_change_in_prod".to_string());

    let (join, cancel) = discovery::serve();
    if let Err(e) = server::main(apikey).await {
        log::error!("Failed to start server: {}", e);
        cancel.cancel();
    }
    join.await?;
    Ok(())
}
