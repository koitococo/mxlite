use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::states::SharedAppState;
use axum::extract::Path;

pub(crate) fn build(app: SharedAppState) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/file/{name}", get(get_file))
}

#[derive(Deserialize)]
struct GetFileParams {
    xxh3: Option<bool>,
    sha1: Option<bool>,
}

async fn get_file(
    State(app): State<SharedAppState>,
    Path(name): Path<String>,
    Query(params): Query<GetFileParams>,
) -> Response {
    let map = app
        .file_map
        .get_file_with_optional_props(
            &name,
            params.xxh3.unwrap_or(false),
            params.sha1.unwrap_or(false),
        )
        .await;
    if let Some(file_map) = map {
        let file_path = file_map.file_path.clone();
        let file = File::open(file_path).await;
        if let Ok(file) = file {
            let builder = Response::builder()
                .header("Content-Type", "application/octet-stream")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", name),
                );
            let builder = if let Some(xxh3) = file_map.xxh3 {
                builder.header("X-Hash-Xxh3", xxh3)
            } else {
                builder
            };
            let builder = if let Some(sha1) = file_map.sha1 {
                builder.header("X-Hash-Sha1", sha1)
            } else {
                builder
            };
            let builder = builder.body(Body::from_stream(ReaderStream::new(file)));
            if let Ok(response) = builder {
                return response;
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json("Failed to create response"),
                )
                    .into_response()
            }
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Failed to open file"),
            )
                .into_response()
        }
    } else {
        (StatusCode::NOT_FOUND, Json("File not found")).into_response()
    }
}
