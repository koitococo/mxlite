use axum::{Json, Router, extract::State, routing::method_routing};
use futures_util::future::join_all;
use serde::Serialize;

use crate::{daemon::states::{host_session::ExtraInfo, SharedAppState}, utils::states::States as _};

#[derive(Serialize)]
struct GetRespInner {
  host: String,
  info: Option<ExtraInfo>,
}

#[derive(Serialize)]
struct GetResponse {
  ok: bool,
  hosts: Vec<GetRespInner>,
}

async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
  let hosts = join_all(app.host_session.list().iter().map(async |s| GetRespInner {
    host: s.clone(),
    info: app.host_session.get(s).map(|s| s.extra.clone()),
  }))
  .await;
  Json(GetResponse { ok: true, hosts })
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get))
}
