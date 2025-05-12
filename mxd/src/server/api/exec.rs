use axum::{Json, Router, extract::State, http::StatusCode, routing::method_routing};
use common::protocol::controller::CommandExecutionRequest;
use serde::Deserialize;

use crate::states::SharedAppState;

use super::{SendReqResponse, send_req_helper};

#[derive(Deserialize)]
struct PostRequest {
  host: String,
  cmd: String,
  args: Option<Vec<String>>,
  use_script: Option<bool>,
  use_shell: Option<bool>,
}

async fn post(
  State(app): State<SharedAppState>, Json(params): Json<PostRequest>,
) -> (StatusCode, Json<SendReqResponse>) {
  send_req_helper(
    app,
    params.host,
    CommandExecutionRequest {
      command: params.cmd,
      args: params.args,
      use_script_file: params.use_script,
      use_shell: params.use_shell,
    }
    .into(),
  )
  .await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::post(post))
}
