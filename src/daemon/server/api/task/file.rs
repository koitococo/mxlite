use crate::protocol::messaging::{FileDownloadParams, FileUploadParams};
use axum::{Json, Router, extract::State, http::StatusCode, routing::method_routing};
use serde::Deserialize;

use crate::daemon::states::SharedAppState;

use super::utils::{SendReqResponse, send_req_helper};

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
    match params.op {
      FileOperation::Download => FileDownloadParams {
        src_url: params.url,
        dest_path: params.path,
      }
      .into(),
      FileOperation::Upload => FileUploadParams {
        src_path: params.path,
        dest_url: params.url,
      }
      .into(),
    },
  )
  .await
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::post(post))
}
