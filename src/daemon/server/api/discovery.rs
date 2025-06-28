use axum::{Json, Router, extract::State, routing::method_routing};
use serde::{Deserialize, Serialize};

use crate::daemon::states::SharedAppState;

#[derive(Serialize)]
struct GetResponse {
  enabled: bool,
  running: bool,
}

async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
  let (enabled, running) = if let Some(ds) = app.discovery_service.as_ref() {
    let ds = ds.lock().await;
    (true, ds.running())
  } else {
    (false, false)
  };
  Json(GetResponse { enabled, running })
}

#[derive(Deserialize)]
struct PostRequest {
  start: bool,
}

#[derive(Serialize)]
struct PostResponse {
  ok: bool,
  state: GetResponse,
}

#[axum::debug_handler]
async fn post(State(app): State<SharedAppState>, Json(params): Json<PostRequest>) -> Json<PostResponse> {
  let (ok, enabled) = if let Some(ds) = app.discovery_service.as_ref() {
    let mut ds = ds.lock().await;
    let ok = if params.start {
      if let Err(e) = ds.start() {
        log::error!("Failed to start discovery service: {e}");
        false
      } else {
        true
      }
    } else {
      if let Err(e) = ds.stop().await {
        log::error!("Failed to stop discovery service: {e}");
        false
      } else {
        true
      }
    };
    (ok, true)
  } else {
    log::warn!("Discovery service is not available");
    (false, false)
  };
  Json(PostResponse {
    ok,
    state: GetResponse {
      enabled,
      running: params.start ^ !ok,
    },
  })
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get).post(post))
}
