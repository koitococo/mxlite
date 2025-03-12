use discovery::discover_controller;
use log::{error, info, warn};

mod discovery;
mod executor;
mod net;
mod utils;

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .env()
        .with_utc_timestamps()
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
                "ws://controller.local:8080/ws".to_string()
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
