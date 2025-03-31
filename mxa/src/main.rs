use anyhow::{Result, anyhow};
use discovery::discover_controller;
use log::{LevelFilter, error, info, warn};

mod discovery;
mod executor;
mod net;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_utc_timestamps()
        .env()
        .init()
        .unwrap();

    info!("MetalX Agent - Launching");
    let host_id = match utils::get_machine_id() {
        Ok(id) => id,
        Err(err) => {
            error!("Failed to get machine id: {}", err);
            if cfg!(debug_assertions) {
                eprintln!(
                    "[Debug] Failed to get machine id, use 'cafecafecafecafecafecafecafecafe' instead",
                );
                "cafecafecafecafecafecafecafecafe".to_string()
            } else {
                std::process::exit(1);
            }
        }
    };
    let ws_url = match std::env::var("WS_URL") {
        Ok(url) => url,
        Err(_) => {
            let controllers = match discover_controller().await {
                Ok(c) => c,
                Err(err) => {
                    error!("Failed to discover controller: {}", err);
                    std::process::exit(1);
                }
            };
            if controllers.is_empty() {
                warn!("No controller discovered");
                return Err(anyhow!("Failed to discover controller"));
            } else {
                controllers[0].clone()
            }
        }
    };
    loop {
        if let Err(err) = net::handle_ws_url(ws_url.clone(), host_id.clone()).await {
            error!("Agent failed: {}", err);
        }
    }
}
