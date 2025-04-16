use axum::{
  Json,
  extract::{Query, State},
  http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::states::{SharedAppState, host_session::ExtraInfo};

#[derive(Deserialize)]
pub(super) struct GetParams {
  host: String,
}

#[derive(Serialize)]
pub(super) struct GetResponse {
  ok: bool,
  host: String,
  info: Option<ExtraInfo>,
}

pub(super) async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> (StatusCode, Json<GetResponse>) {
  if let Some(info) = app.host_session.get(&params.host).map(|s| s.extra.clone()) {
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
