pub mod file_map;
pub mod host_session;

use std::sync::Arc;

use file_map::FileMapStorage;
use host_session::HostSessionStorage;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::daemon::cli::StartupArgs;

pub struct AppState {
  pub host_session: HostSessionStorage,
  pub file_map: FileMapStorage,
  pub cancel_signal: CancellationToken,
  pub startup_args: StartupArgs,
  pub discovery_service: Option<Mutex<crate::daemon::discovery::DiscoveryService>>,
}

impl AppState {
  pub fn new(startup_args: StartupArgs) -> Self {
    AppState {
      host_session: HostSessionStorage::new(),
      file_map: FileMapStorage::new(),
      cancel_signal: CancellationToken::new(),
      startup_args,
      discovery_service: None,
    }
  }
}

pub type SharedAppState = Arc<AppState>;
