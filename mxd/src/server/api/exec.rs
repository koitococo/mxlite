use axum::{extract::State, http::StatusCode, routing::method_routing, Json, Router};
use common::protocol::controller::{CommandExecutionRequest, ControllerRequest, ControllerRequestPayload, PROTOCOL_VERSION};
use serde::Deserialize;

use crate::states::SharedAppState;

use super::{SendReqResponse, send_req_helper};

#[derive(Deserialize)]
struct PostRequest {
  host: String,
  cmd: String,
  use_script: Option<bool>,
}

async fn post(State(app): State<SharedAppState>, Json(params): Json<PostRequest>) -> (StatusCode, Json<SendReqResponse>) {
  send_req_helper(
    app,
    params.host,
    ControllerRequest {
      version: PROTOCOL_VERSION,
      id: 0,
      payload: ControllerRequestPayload::CommandExecutionRequest(CommandExecutionRequest {
        command: params.cmd,
        use_script_file: params.use_script.unwrap_or(false),
      }),
    },
  )
  .await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> { Router::new().with_state(app.clone()).route("/", method_routing::post(post)) }
