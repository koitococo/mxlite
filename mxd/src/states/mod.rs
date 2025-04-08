pub(crate) mod file_map;
pub(crate) mod host_session;

use std::sync::Arc;

use file_map::FileMapStorage;
use host_session::HostSessionStorage;
use tokio_util::sync::CancellationToken;

pub(crate) struct AppState {
    pub(crate) host_session: HostSessionStorage,
    pub(crate) file_map: FileMapStorage,
    pub(crate) cancel_signal: CancellationToken,
}

impl AppState {
    fn new(cancel_signal: CancellationToken) -> Self {
        AppState {
            host_session: HostSessionStorage::new(),
            file_map: FileMapStorage::new(),
            cancel_signal,
        }
    }
}

pub(crate) type SharedAppState = Arc<AppState>;
pub(crate) fn new_shared_app_state(ct: CancellationToken) -> SharedAppState {
    Arc::new(AppState::new(ct))
}
