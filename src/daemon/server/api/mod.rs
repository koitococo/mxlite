mod all_task;
mod file_map;
mod fs;
mod info;
mod list;
mod list_info;
mod relative_url;
mod result;
mod task;
mod discovery;

use axum::Router;

use crate::daemon::states::SharedAppState;

use super::utils::auth_middleware;

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  let router = Router::new()
    .with_state(app.clone())
    .nest("/all-tasks", self::all_task::build(app.clone()))
    .nest("/discovery", self::discovery::build(app.clone()))
    .nest("/file-map", self::file_map::build(app.clone()))
    .nest("/fs", self::fs::build(app.clone()))
    .nest("/list", self::list::build(app.clone()))
    .nest("/list-info", self::list_info::build(app.clone()))
    .nest("/info", self::info::build(app.clone()))
    .nest("/relative-url", self::relative_url::build(app.clone()))
    .nest("/result", self::result::build(app.clone()))
    .nest("/task", self::task::build(app.clone()));
  auth_middleware(router, app.startup_args.apikey.clone())
}
