use crate::{
  daemon::{
    server::api::{ERR_REASON_INTERNAL_ERROR, ERR_REASON_SESSION_NOT_FOUND},
    states::{SharedAppState, host_session::HostSessionStorageExt as _},
  },
  protocol::messaging::ControllerRequestPayload,
};
use axum::{Json, http::StatusCode};
use log::error;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct SendReqResponse {
  ok: bool,
  task_id: Option<u32>,
  reason: Option<String>,
}

pub(super) async fn send_req_helper(
  app: SharedAppState, host: String, req: ControllerRequestPayload,
) -> (StatusCode, Json<SendReqResponse>) {
  if let Some(r) = app.host_session.send_request(&host, req).await {
    match r {
      Ok(req_id) => (
        StatusCode::OK,
        Json(SendReqResponse {
          ok: true,
          task_id: Some(req_id),
          reason: None,
        }),
      ),
      Err(e) => {
        error!("Failed to pass internal message to host session: {} {:?}", &host, e);
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(SendReqResponse {
            ok: false,
            task_id: None,
            reason: Some(ERR_REASON_INTERNAL_ERROR.to_string()),
          }),
        )
      }
    }
  } else {
    (
      StatusCode::NOT_FOUND,
      Json(SendReqResponse {
        ok: false,
        task_id: None,
        reason: Some(ERR_REASON_SESSION_NOT_FOUND.to_string()),
      }),
    )
  }
}
