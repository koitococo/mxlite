use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::states::SharedAppState;
use axum::extract::Path;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/{name}", get(get_file).head(head_file))
}

#[derive(Deserialize)]
struct GetFileParams {
    xxh3: Option<bool>,
    sha1: Option<bool>,
    sha256: Option<bool>,
    sha512: Option<bool>,
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
            params.sha256.unwrap_or(false),
            params.sha512.unwrap_or(false),
        )
        .await;
    if let Some(file_map) = map {
        let file_path = file_map.file_path.clone();
        let file = File::open(file_path).await;
        if let Ok(file) = file {
            let mut builder = Response::builder()
                .header("Content-Type", "application/octet-stream")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", name),
                );
            if let Some(xxh3) = file_map.xxh3 {
                builder = builder.header("X-Hash-Xxh3", xxh3);
            }
            if let Some(sha1) = file_map.sha1 {
                builder = builder.header("X-Hash-Sha1", sha1);
            }
            if let Some(sha256) = file_map.sha256 {
                builder = builder.header("X-Hash-Sha256", sha256);
            }
            if let Some(sha512) = file_map.sha512 {
                builder = builder.header("X-Hash-Sha512", sha512);
            }
            let builder = builder.body(Body::from_stream(ReaderStream::new(file)));
            if let Ok(response) = builder {
                response
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

async fn head_file(
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
            params.sha256.unwrap_or(false),
            params.sha512.unwrap_or(false),
        )
        .await;
    if let Some(file_map) = map {
        let mut response = StatusCode::NO_CONTENT.into_response();
        let headers = response.headers_mut();
        if let Some(xxh3) = file_map.xxh3 {
            headers.append(
                "X-Hash-Xxh3",
                xxh3.parse().unwrap_or(HeaderValue::from_static("")),
            );
        }
        if let Some(sha1) = file_map.sha1 {
            headers.append(
                "X-Hash-Sha1",
                sha1.parse().unwrap_or(HeaderValue::from_static("")),
            );
        }
        response
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
