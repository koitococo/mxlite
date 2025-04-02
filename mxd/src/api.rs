use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
};
use common::messages::{
    AgentResponse, CommandExecutionRequest, ControllerRequest, ControllerRequestPayload,
    FileTransferRequest, PROTOCOL_VERSION,
};
use log::error;
use serde::{Deserialize, Serialize};

use crate::states::{ExtraInfo, SharedAppState, TaskState};

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

#[derive(Clone)]
struct ApiState {
    apikey: String,
}

pub(crate) fn build(app: SharedAppState, apikey: String) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/list", get(get_list))
        .route("/info", get(get_info))
        .route("/result", get(get_result))
        .route("/exec", post(post_exec))
        .route("/file", post(post_file))
        .route(
            "/add_file",
            post(post_add_file)
                .get(get_add_file)
                .delete(delete_add_file),
        )
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

async fn get_list(State(app): State<SharedAppState>) -> Json<GetListResponse> {
    let sessions = app.list_sessions().await;
    Json(GetListResponse { sessions })
}

#[derive(Deserialize)]
struct GetInfoParams {
    host: String,
}

#[derive(Serialize)]
struct GetInfoResponse {
    ok: bool,
    host: String,
    info: Option<ExtraInfo>,
}

async fn get_info(
    State(app): State<SharedAppState>,
    params: Query<GetInfoParams>,
) -> (StatusCode, Json<GetInfoResponse>) {
    if let Some(info) = app.get_extra_info(&params.host).await {
        (
            StatusCode::OK,
            Json(GetInfoResponse {
                ok: true,
                host: params.host.clone(),
                info: Some(info),
            }),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(GetInfoResponse {
                ok: false,
                host: params.host.clone(),
                info: None,
            }),
        )
    }
}

#[derive(Deserialize)]
struct GetResultParams {
    host: String,
    task_id: u64,
}

#[derive(Serialize)]
struct GetResultResponse {
    ok: bool,
    payload: Option<AgentResponse>,
    reason: Option<String>,
}

async fn get_result(
    State(app): State<SharedAppState>,
    params: Query<GetResultParams>,
) -> (StatusCode, Json<GetResultResponse>) {
    if let Some(state) = app.get_resp(&params.host, params.task_id).await {
        if let Some(state) = state {
            if let TaskState::Finished(resp) = state {
                (
                    StatusCode::OK,
                    Json(GetResultResponse {
                        ok: true,
                        payload: Some(resp),
                        reason: None,
                    }),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(GetResultResponse {
                        ok: false,
                        payload: None,
                        reason: Some(ERR_REASON_TASK_NOT_COMPLETED.to_string()),
                    }),
                )
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(GetResultResponse {
                    ok: false,
                    payload: None,
                    reason: Some(ERR_REASON_TASK_NOT_FOUND.to_string()),
                }),
            )
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(GetResultResponse {
                ok: false,
                payload: None,
                reason: Some(ERR_REASON_SESSION_NOT_FOUND.to_string()),
            }),
        )
    }
}

#[derive(Serialize)]
struct SendReqResponse {
    ok: bool,
    task_id: Option<u64>,
    reason: Option<String>,
}

async fn send_req_helper(
    app: SharedAppState,
    host: String,
    req: ControllerRequest,
) -> (StatusCode, Json<SendReqResponse>) {
    if let Some(r) = app.send_req(&host, req).await {
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
                error!(
                    "Failed to pass internal message to host session: {} {:?}",
                    &host, e
                );
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

#[derive(Deserialize)]
struct PostExecRequest {
    host: String,
    cmd: String,
}

async fn post_exec(
    State(app): State<SharedAppState>,
    Json(params): Json<PostExecRequest>,
) -> (StatusCode, Json<SendReqResponse>) {
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
                    FileOperation::Download => common::messages::FileOperation::Download,
                    FileOperation::Upload => common::messages::FileOperation::Upload,
                },
            }),
        },
    )
    .await
}

#[derive(Deserialize)]
struct PostAddFileRequest {
    path: String,
}

async fn post_add_file(
    State(app): State<SharedAppState>,
    Json(params): Json<PostAddFileRequest>,
) -> (StatusCode, Json<String>) {
    match app.add_file(params.path).await {
        Ok(hash) => (StatusCode::OK, Json(hash.clone())),
        Err(e) => {
            error!("Failed to add file: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to add file".to_string()),
            )
        }
    }
}

#[derive(Serialize)]
struct GetAddFileResponse {
    files: Vec<String>,
}

async fn get_add_file(State(app): State<SharedAppState>) -> Json<GetAddFileResponse> {
    Json(GetAddFileResponse {
        files: app.get_all_files().await,
    })
}

#[derive(Deserialize)]
struct DeleteAddFileRequest {
    hash: String,
}

async fn delete_add_file(
    State(app): State<SharedAppState>,
    Query(params): Query<DeleteAddFileRequest>,
) -> StatusCode {
    if app.get_file(&params.hash).await.is_none() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    }
}
