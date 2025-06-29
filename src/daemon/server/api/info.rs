use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  routing::method_routing,
};
use serde::{Deserialize, Serialize};

use crate::{daemon::states::{host_session::ExtraInfo, SharedAppState}, utils::states::States as _};

#[derive(Deserialize)]
struct GetParams {
  host: String,
}

#[derive(Serialize)]
struct GetResponse {
  ok: bool,
  host: String,
  info: Option<ExtraInfo>,
}

async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> (StatusCode, Json<GetResponse>) {
  if let Some(info) = app.host_session.get_arc(&params.host).map(|s| s.extra.clone()) {
    (
      StatusCode::OK,
      Json(GetResponse {
        ok: true,
        host: params.host.clone(),
        info: Some(info),
      }),
    )
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(GetResponse {
        ok: false,
        host: params.host.clone(),
        info: None,
      }),
    )
  }
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get))
}
