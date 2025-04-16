mod all_task;
mod exec;
mod file;
mod file_map;
mod info;
mod list;
mod list_info;
mod result;

use axum::{
  Json, Router,
  extract::{Request, State},
  http::StatusCode,
  middleware::{self, Next},
  response::IntoResponse,
  routing::{get, post},
};
use common::protocol::controller::ControllerRequest;
use log::error;
use serde::Serialize;

use crate::states::SharedAppState;

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

#[derive(Clone)]
struct ApiState {
  apikey: Option<String>,
}

pub(crate) fn auth_middleware<T: Clone + Send + Sync + 'static>(router: Router<T>, key: Option<String>) -> Router<T> {
  router.layer(middleware::from_fn_with_state(
    ApiState {
      apikey: key.map(|sk| format!("Bearer {}", sk)),
    },
    async |State(state): State<ApiState>, request: Request, next: Next| {
      if let Some(sk) = state.apikey {
        if let Some(key) = request.headers().get("Authorization") {
          if key != &sk {
            return (StatusCode::FORBIDDEN).into_response();
          }
        } else {
          return (StatusCode::UNAUTHORIZED).into_response();
        }
      }
      next.run(request).await
    },
  ))
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  let router = Router::new()
    .with_state(app.clone())
    .route("/list", get(list::get))
    .route("/list-info", get(list_info::get))
    .route("/info", get(info::get))
    .route("/all-tasks", get(all_task::get))
    .route("/result", get(result::get))
    .route("/exec", post(exec::post))
    .route("/file", post(file::post))
    .route("/file-map", post(file_map::post).get(file_map::get).delete(file_map::delete));
  auth_middleware(router, app.startup_args.apikey.clone())
}

#[derive(Serialize)]
struct SendReqResponse {
  ok: bool,
  task_id: Option<u64>,
  reason: Option<String>,
}

async fn send_req_helper(app: SharedAppState, host: String, req: ControllerRequest) -> (StatusCode, Json<SendReqResponse>) {
  if let Some(r) = app.host_session.send_req(&host, req).await {
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
