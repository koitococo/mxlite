use axum::{Json, Router, extract::State, routing::method_routing};
use serde::Serialize;

use crate::states::SharedAppState;

#[derive(Serialize)]
struct GetResponse {
  ok: bool,
  sessions: Vec<String>,
}

async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
  Json(GetResponse {
    ok: true,
    sessions: app.host_session.list(),
  })
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> { Router::new().with_state(app.clone()).route("/", method_routing::get(get)) }
