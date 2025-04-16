use axum::{Json, extract::State, http::StatusCode};
use common::protocol::controller::{self, ControllerRequest, ControllerRequestPayload, FileTransferRequest, PROTOCOL_VERSION};
use serde::Deserialize;

use crate::states::SharedAppState;

use super::{SendReqResponse, send_req_helper};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum FileOperation {
  Download,
  Upload,
}

#[derive(Deserialize)]
pub(super) struct PostRequest {
  url: String,
  path: String,
  host: String,
  op: FileOperation,
}

pub(super) async fn post(State(app): State<SharedAppState>, Json(params): Json<PostRequest>) -> (StatusCode, Json<SendReqResponse>) {
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
