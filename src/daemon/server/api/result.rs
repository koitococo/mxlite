use crate::protocol::messaging::AgentResponse;
use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  routing::method_routing,
};
use serde::{Deserialize, Serialize};

use crate::daemon::states::{SharedAppState, host_session::TaskState};

use super::{ERR_REASON_SESSION_NOT_FOUND, ERR_REASON_TASK_NOT_COMPLETED, ERR_REASON_TASK_NOT_FOUND};

#[derive(Deserialize)]
struct GetParams {
  host: String,
  task_id: u64,
}

#[derive(Serialize)]
struct GetResponse {
  ok: bool,
  payload: Option<AgentResponse>,
  reason: Option<String>,
}

async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> (StatusCode, Json<GetResponse>) {
  if let Some(state) = app.host_session.get_resp(&params.host, params.task_id).await {
    if let Some(state) = state {
      if let TaskState::Finished(resp) = state {
        (
          StatusCode::OK,
          Json(GetResponse {
            ok: true,
            payload: Some(resp),
            reason: None,
          }),
        )
      } else {
        (
          StatusCode::NOT_FOUND,
          Json(GetResponse {
            ok: false,
            payload: None,
            reason: Some(ERR_REASON_TASK_NOT_COMPLETED.to_string()),
          }),
        )
      }
    } else {
      (
        StatusCode::NOT_FOUND,
        Json(GetResponse {
          ok: false,
          payload: None,
          reason: Some(ERR_REASON_TASK_NOT_FOUND.to_string()),
        }),
      )
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(GetResponse {
        ok: false,
        payload: None,
        reason: Some(ERR_REASON_SESSION_NOT_FOUND.to_string()),
      }),
    )
  }
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get))
}
