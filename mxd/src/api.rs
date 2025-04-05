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
use futures_util::future::join_all;
use log::error;
use serde::{Deserialize, Serialize};

use crate::states::{
    SharedAppState,
    host_session::{ExtraInfo, TaskState},
};

const ERR_REASON_SESSION_NOT_FOUND: &str = "SESSION_NOT_FOUND";
const ERR_REASON_TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
const ERR_REASON_TASK_NOT_COMPLETED: &str = "TASK_NOT_COMPLETED";
const ERR_REASON_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

#[derive(Clone)]
struct ApiState {
    apikey: Option<String>,
}

pub(crate) fn build(app: SharedAppState, apikey: Option<String>) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/list", get(get_list))
        .route("/list-info", get(get_list_info))
        .route("/info", get(get_info))
        .route("/result", get(get_result))
        .route("/exec", post(post_exec))
        .route("/file", post(post_file))
        .route(
            "/file-map",
            post(post_file_map)
                .get(get_file_map)
                .delete(delete_file_map),
        )
        .layer(middleware::from_fn_with_state(
            ApiState {
                apikey: apikey.map(|sk| format!("Bearer {}", sk)),
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

#[derive(Serialize)]
struct GetListResponse {
    ok: bool,
    sessions: Vec<String>,
}

async fn get_list(State(app): State<SharedAppState>) -> Json<GetListResponse> {
    let sessions = app.host_session.list_sessions().await;
    Json(GetListResponse { ok: true, sessions })
}

#[derive(Serialize)]
struct GetListInfoInner {
    host: String,
    info: Option<ExtraInfo>,
}

#[derive(Serialize)]
struct GetListInfoResponse {
    ok: bool,
    hosts: Vec<GetListInfoInner>,
}

async fn get_list_info(State(app): State<SharedAppState>) -> Json<GetListInfoResponse> {
    let hosts = join_all(
        app.host_session
            .list_sessions()
            .await
            .iter()
            .map(async |s| GetListInfoInner {
                host: s.clone(),
                info: app.host_session.get_extra_info(s).await,
            }),
    )
    .await;
    Json(GetListInfoResponse { ok: true, hosts })
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
    if let Some(info) = app.host_session.get_extra_info(&params.host).await {
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
    if let Some(state) = app
        .host_session
        .get_resp(&params.host, params.task_id)
        .await
    {
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
    use_script: Option<bool>,
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
                use_script_file: params.use_script.unwrap_or(false),
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
    publish_name: String,
}

async fn post_file_map(
    State(app): State<SharedAppState>,
    Json(params): Json<PostAddFileRequest>,
) -> (StatusCode, Json<String>) {
    if let Err(e) = app
        .file_map
        .add_file_map(params.path, params.publish_name)
        .await
    {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(e))
    } else {
        (
            StatusCode::OK,
            Json("File map added successfully".to_string()),
        )
    }
}

#[derive(Serialize)]
struct GetAddFileResponse {
    files: Vec<String>,
}

async fn get_file_map(State(app): State<SharedAppState>) -> Json<GetAddFileResponse> {
    Json(GetAddFileResponse {
        files: app.file_map.get_all_files().await,
    })
}

#[derive(Deserialize)]
struct DeleteAddFileRequest {
    publish_name: String,
}

async fn delete_file_map(
    State(app): State<SharedAppState>,
    Query(params): Query<DeleteAddFileRequest>,
) -> StatusCode {
    app.file_map.del_file_map(&params.publish_name).await;
    StatusCode::OK
}
