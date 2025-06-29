use crate::{protocol::messaging::AgentResponse, utils::states::States};
use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  routing::method_routing,
};
use serde::{Deserialize, Serialize};

use crate::daemon::states::{SharedAppState};

use super::{ERR_REASON_SESSION_NOT_FOUND, ERR_REASON_TASK_NOT_COMPLETED, ERR_REASON_TASK_NOT_FOUND};

#[derive(Deserialize)]
struct GetParams {
  host: String,
  task_id: u32,
}

#[derive(Serialize)]
struct GetResponse {
  ok: bool,
  payload: Option<AgentResponse>,
  reason: Option<String>,
}

async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> (StatusCode, Json<GetResponse>) {
  let Some(session) = app.host_session.get_arc(&params.host) else {
    return (
      StatusCode::NOT_FOUND,
      Json(GetResponse {
        ok: false,
        payload: None,
        reason: Some(ERR_REASON_SESSION_NOT_FOUND.to_string()),
      }),
    );
  };

  let Some(task) = session.tasks.take_if(params.task_id, |v| v.is_some()) else {
    return (
      StatusCode::NOT_FOUND,
      Json(GetResponse {
        ok: false,
        payload: None,
        reason: Some(ERR_REASON_TASK_NOT_FOUND.to_string()),
      }),
    );
  };

  let Some(resp) = task.as_ref() else {
    return (
      StatusCode::NOT_FOUND,
      Json(GetResponse {
        ok: false,
        payload: None,
        reason: Some(ERR_REASON_TASK_NOT_COMPLETED.to_string()),
      }),
    );
  };

  (
    StatusCode::OK,
    Json(GetResponse {
      ok: true,
      payload: Some(resp.clone()),
      reason: None,
    }),
  )
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get))
}
