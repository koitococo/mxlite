use axum::{Json, Router, extract::State, routing::method_routing};
use serde::Serialize;

use crate::{daemon::states::SharedAppState, utils::states::States as _};

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

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app).route("/", method_routing::get(get))
}
