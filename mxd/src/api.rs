use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
};
use common::messages::{
    CommandExecutionRequest, ControllerRequest, ControllerRequestPayload, FileTransferRequest,
    PROTOCOL_VERSION,
};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::states::{SharedAppState, TaskState};

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

#[derive(Clone)]
struct ApiState {
    apikey: String,
}

pub(crate) fn build_api(app: SharedAppState, apikey: String) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/list", get(get_list))
        .route("/result", get(get_result))
        .route("/exec", post(post_exec))
        .route("/file", post(post_file))
        .layer(middleware::from_fn_with_state(
            ApiState {
                apikey: format!("Bearer {}", apikey),
            },
            async |State(state): State<ApiState>, request: Request, next: Next| {
                let headers = request.headers();
                if let Some(key) = headers.get("Authorization") {
                    if key != &state.apikey {
                        return (StatusCode::FORBIDDEN).into_response();
                    }
                    next.run(request).await
                } else {
                    (StatusCode::UNAUTHORIZED).into_response()
                }
            },
        ))
}

#[derive(Serialize)]
struct GetListResponse {
    sessions: Vec<String>,
}

async fn get_list(State(app): State<SharedAppState>) -> impl IntoResponse {
    let sessions = app.list_sessions().await;
    Json(GetListResponse { sessions })
}

#[derive(Deserialize)]
struct GetResultParams {
    host: String,
    task_id: u64,
}

async fn get_result(
    State(app): State<SharedAppState>,
    params: Query<GetResultParams>,
) -> impl IntoResponse {
    if let Some(state) = app.get_resp(&params.host, params.task_id).await {
        if let Some(state) = state {
            if let TaskState::Finished(resp) = state {
                (
                    StatusCode::OK,
                    Json(json!({
                        "ok": resp.ok,
                        "payload": resp.payload
                    })),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "ok": false,
                        "reason": ERR_REASON_TASK_NOT_COMPLETED
                    })),
                )
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "ok": false,
                    "reason": ERR_REASON_TASK_NOT_FOUND
                })),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "reason": ERR_REASON_SESSION_NOT_FOUND
            })),
        )
    }
}

async fn send_req_helper(
    app: SharedAppState,
    host: String,
    req: ControllerRequest,
) -> impl IntoResponse {
    if let Some(r) = app.send_req(&host, req).await {
        match r {
            Ok(req_id) => (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "task_id": req_id
                })),
            ),
            Err(e) => {
                error!(
                    "Failed to pass internal message to host session: {} {:?}",
                    &host, e
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "ok": false,
                        "reason": ERR_REASON_INTERNAL_ERROR
                    })),
                )
            }
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "reason": ERR_REASON_SESSION_NOT_FOUND
            })),
        )
    }
}

#[derive(Deserialize)]
struct PostExecRequest {
    host: String,
    cmd: String,
}

async fn post_exec(
    State(app): State<SharedAppState>,
    Json(params): Json<PostExecRequest>,
) -> impl IntoResponse {
    send_req_helper(
        app,
        params.host,
        ControllerRequest {
            version: PROTOCOL_VERSION,
            id: 0,
            payload: ControllerRequestPayload::CommandExecutionRequest(CommandExecutionRequest {
                command: params.cmd,
            }),
        },
    )
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileOperation {
    Download,
    Upload,
}

#[derive(Deserialize)]
struct PostFileRequest {
    url: String,
    path: String,
    host: String,
    op: FileOperation,
}

async fn post_file(
    State(app): State<SharedAppState>,
    Json(params): Json<PostFileRequest>,
) -> impl IntoResponse {
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
                    FileOperation::Download => common::messages::FileOperation::Download,
                    FileOperation::Upload => common::messages::FileOperation::Upload,
                },
            }),
        },
    )
    .await
}
