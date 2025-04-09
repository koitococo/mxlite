use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::states::SharedAppState;

#[derive(Deserialize)]
pub(super) struct PostRequest {
    path: String,
    publish_name: String,
}

pub(super) async fn post(
    State(app): State<SharedAppState>,
    Json(params): Json<PostRequest>,
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
pub(super) struct GetResponse {
    files: Vec<String>,
}

pub(super) async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
    Json(GetResponse {
        files: app.file_map.get_all_files(),
    })
}

#[derive(Deserialize)]
pub(super) struct DeleteRequest {
    publish_name: String,
}

pub(super) async fn delete(
    State(app): State<SharedAppState>,
    Query(params): Query<DeleteRequest>,
) -> StatusCode {
    app.file_map.del_file_map(&params.publish_name);
    StatusCode::OK
}
