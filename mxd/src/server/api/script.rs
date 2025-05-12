use axum::{Json, Router, extract::State, http::StatusCode, routing::method_routing};
use common::protocol::controller::ScriptEvalRequest;
use serde::Deserialize;

use crate::states::SharedAppState;

use super::{SendReqResponse, send_req_helper};

#[derive(Deserialize)]
struct PostRequest {
  host: String,
  script: String,
}

async fn post(
  State(app): State<SharedAppState>, Json(params): Json<PostRequest>,
) -> (StatusCode, Json<SendReqResponse>) {
  send_req_helper(app, params.host, ScriptEvalRequest { script: params.script }.into()).await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::post(post))
}
