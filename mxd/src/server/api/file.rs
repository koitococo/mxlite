use axum::{Json, Router, extract::State, http::StatusCode, routing::method_routing};
use common::protocol::controller::{
  self, ControllerRequest, ControllerRequestPayload, FileTransferRequest, PROTOCOL_VERSION,
};
use serde::Deserialize;

use crate::states::SharedAppState;

use super::{SendReqResponse, send_req_helper};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileOperation {
  Download,
  Upload,
}

#[derive(Deserialize)]
struct PostRequest {
  url: String,
  path: String,
  host: String,
  op: FileOperation,
}

async fn post(
  State(app): State<SharedAppState>, Json(params): Json<PostRequest>,
) -> (StatusCode, Json<SendReqResponse>) {
  send_req_helper(
    app,
    params.host,
    ControllerRequest {
      version: PROTOCOL_VERSION,
      id: 0,
      payload: ControllerRequestPayload::FileTransferRequest(FileTransferRequest {
        url: params.url,
        path: params.path,
        operation: match params.op {
          FileOperation::Download => controller::FileOperation::Download,
          FileOperation::Upload => controller::FileOperation::Upload,
        },
      }),
    },
  )
  .await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::post(post))
}
