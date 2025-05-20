use axum::{Json, Router, extract::State, http::StatusCode, routing::method_routing};
use common::protocol::messaging::{self, FileTransferRequest};
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
    FileTransferRequest {
      url: params.url,
      path: params.path,
      operation: match params.op {
        FileOperation::Download => messaging::FileOperation::Download,
        FileOperation::Upload => messaging::FileOperation::Upload,
      },
    }
    .into(),
  )
  .await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::post(post))
}
