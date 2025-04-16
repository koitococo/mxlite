pub(crate) mod file_map;
pub(crate) mod host_session;

use std::sync::Arc;

use file_map::FileMapStorage;
use host_session::HostSessionStorage;
use tokio_util::sync::CancellationToken;

use crate::StartupArguments;

pub(crate) struct AppState {
  pub(crate) host_session: HostSessionStorage,
  pub(crate) file_map: FileMapStorage,
  pub(crate) cancel_signal: CancellationToken,
  pub(crate) startup_args: StartupArguments,
}

impl AppState {
  pub(crate) fn new(cancel_signal: CancellationToken, startup_args: StartupArguments) -> Self {
    AppState {
      host_session: HostSessionStorage::new(),
      file_map: FileMapStorage::new(),
      cancel_signal,
      startup_args,
    }
  }
}

pub(crate) type SharedAppState = Arc<AppState>;
