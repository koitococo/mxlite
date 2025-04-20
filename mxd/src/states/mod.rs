pub(crate) mod file_map;
pub(crate) mod host_session;

use std::sync::Arc;

use file_map::FileMapStorage;
use host_session::HostSessionStorage;
use tokio_util::sync::CancellationToken;

use crate::StartupArgs;

pub(crate) struct AppState {
  pub(crate) host_session: HostSessionStorage,
  pub(crate) file_map: FileMapStorage,
  pub(crate) cancel_signal: CancellationToken,
  pub(crate) startup_args: StartupArgs,
}

impl AppState {
  pub(crate) fn new(cancel_signal: CancellationToken, startup_args: StartupArgs) -> Self {
    AppState {
      host_session: HostSessionStorage::new(),
      file_map: FileMapStorage::new(),
      cancel_signal,
      startup_args,
    }
  }
}

pub(crate) type SharedAppState = Arc<AppState>;
