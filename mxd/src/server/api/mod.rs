mod all_task;
mod exec;
mod file;
mod file_map;
mod info;
mod list;
mod list_info;
mod result;
mod script;

use axum::{Json, Router, http::StatusCode};
use common::protocol::messaging::ControllerRequestPayload;
use log::error;
use serde::Serialize;

use crate::states::SharedAppState;

use super::utils::auth_middleware;

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  let router = Router::new()
    .with_state(app.clone())
    .nest("/list", list::build(app.clone()))
    .nest("/list-info", list_info::build(app.clone()))
    .nest("/info", info::build(app.clone()))
    .nest("/all-tasks", all_task::build(app.clone()))
    .nest("/result", result::build(app.clone()))
    .nest("/exec", exec::build(app.clone()))
    .nest("/file", file::build(app.clone()))
    .nest("/file-map", file_map::build(app.clone()))
    .nest("/script", script::build(app.clone()));
  auth_middleware(router, app.startup_args.apikey.clone())
}

#[derive(Serialize)]
struct SendReqResponse {
  ok: bool,
  task_id: Option<u64>,
  reason: Option<String>,
}

async fn send_req_helper(
  app: SharedAppState, host: String, req: ControllerRequestPayload,
) -> (StatusCode, Json<SendReqResponse>) {
  if let Some(r) = app.host_session.send_req(&host, req).await {
    match r {
      Ok(req_id) => (
        StatusCode::OK,
        Json(SendReqResponse {
          ok: true,
          task_id: Some(req_id),
          reason: None,
        }),
      ),
      Err(e) => {
        error!("Failed to pass internal message to host session: {} {:?}", &host, e);
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(SendReqResponse {
            ok: false,
            task_id: None,
            reason: Some(ERR_REASON_INTERNAL_ERROR.to_string()),
          }),
        )
      }
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(SendReqResponse {
        ok: false,
        task_id: None,
        reason: Some(ERR_REASON_SESSION_NOT_FOUND.to_string()),
      }),
    )
  }
}
