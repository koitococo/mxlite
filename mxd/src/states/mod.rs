pub(crate) mod file_map;
pub(crate) mod host_session;

use std::sync::Arc;

use file_map::FileMapStorage;
use host_session::HostSessionStorage;

pub(crate) struct AppState {
    pub(crate) host_session: HostSessionStorage,
    pub(crate) file_map: FileMapStorage,
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {
            host_session: HostSessionStorage::new(),
            file_map: FileMapStorage::new(),
        }
    }
}

pub(crate) type SharedAppState = Arc<AppState>;
pub(crate) fn new_shared_app_state() -> SharedAppState {
    Arc::new(AppState::new())
}
