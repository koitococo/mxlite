mod exec;
mod file;
mod script;
mod utils;

use axum::Router;

use crate::daemon::states::SharedAppState;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new()
    .with_state(app.clone())
    .nest("/exec", self::exec::build(app.clone()))
    .nest("/file", self::file::build(app.clone()))
    .nest("/script", self::script::build(app.clone()))
}
