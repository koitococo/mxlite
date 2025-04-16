use std::fs::metadata;

use axum::{
  Json, Router,
  body::Body,
  extract::{Query, Request, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
  routing::get,
};
use httpdate::HttpDate;
use log::{debug, error, warn};
use serde::Deserialize;
use tokio::{
  fs::File,
  io::{AsyncReadExt, AsyncSeekExt as _},
};
use tokio_util::io::ReaderStream;

use crate::states::{SharedAppState, file_map::FileMap};
use axum::extract::Path;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> { Router::new().with_state(app.clone()).route("/{name}", get(get_file).head(head_file)) }

macro_rules! add_header {
  ($headers:expr, $name:expr, $val:expr) => {
    $headers.append($name, $val.parse().unwrap_or(HeaderValue::from_static("")));
  };
}

#[derive(Deserialize)]
struct GetFileParams {
  xxh3: Option<bool>,
  sha1: Option<bool>,
  sha256: Option<bool>,
  sha512: Option<bool>,
}

async fn get_file(State(app): State<SharedAppState>, Path(name): Path<String>, Query(params): Query<GetFileParams>, req: Request) -> Response {
  debug!("get file: {}", name);
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
  if map.is_none() {
    return StatusCode::NOT_FOUND.into_response();
  }
  let map = map.unwrap();
  match File::open(map.file_path.clone()).await {
    Ok(file) => match file.metadata().await {
      Ok(meta) => {
        let mut builder = Response::builder();
        let headers = builder.headers_mut();
        if headers.is_none() {
          return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let headers = headers.unwrap();
        apply_hash_headers(headers, map);
        add_header!(headers, header::CONTENT_TYPE, "application/octet-stream");
        add_header!(headers, header::ACCEPT_RANGES, "bytes");
        add_header!(headers, header::LAST_MODIFIED, HttpDate::from(meta.modified().unwrap()).to_string());
        let range = req
          .headers()
          .get(header::RANGE)
          .and_then(|v| v.to_str().ok())
          .map(|v| http_range_header::parse_range_header(v).and_then(|v| v.validate(meta.len())));
        match range {
          Some(Ok(range)) => {
            debug!("Range header: {:?}", range);
            if range.len() > 1 {
              warn!("Range header contains multiple ranges: {:?}", range);
              return StatusCode::IM_A_TEAPOT.into_response(); // FIXME: Not supported range
            }
            let start = *range[0].start();
            let end = *range[0].end();
            let len = end - start + 1;
            let builder = builder
              .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, meta.len()))
              .header(header::CONTENT_LENGTH, len.to_string())
              .status(StatusCode::PARTIAL_CONTENT);
            let mut file = file;
            file.seek(std::io::SeekFrom::Start(start)).await.unwrap();
            let stream2 = ReaderStream::with_capacity(file.take(len), 64 * 1024);
            match builder.body(Body::from_stream(stream2)) {
              Ok(response) => response,
              Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to create response: {}", err))).into_response(),
            }
          }
          Some(Err(err)) => (StatusCode::RANGE_NOT_SATISFIABLE, Json(format!("Invalid range header: {}", err))).into_response(),
          None => {
            add_header!(headers, header::CONTENT_LENGTH, meta.len().to_string());
            match builder.body(Body::from_stream(ReaderStream::new(file))) {
              Ok(response) => response,
              Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to create response: {}", err))).into_response(),
            }
          }
        }
      }
      Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to get file metadata: {}", err))).into_response(),
    },
    Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to open file: {}", err))).into_response(),
  }
}

async fn head_file(State(app): State<SharedAppState>, Path(name): Path<String>, Query(params): Query<GetFileParams>) -> Response {
  debug!("head file: {}", name);
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
  if map.is_none() {
    return StatusCode::NOT_FOUND.into_response();
  }
  let map = map.unwrap();
  let meta = metadata(&map.file_path);
  if meta.is_err() {
    error!("Failed to get file metadata: {}", meta.unwrap_err());
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }
  let meta = meta.unwrap();
  let mut response = StatusCode::OK.into_response();
  let headers = response.headers_mut();
  apply_hash_headers(headers, map);
  add_header!(headers, header::CONTENT_LENGTH, meta.len().to_string());
  add_header!(headers, header::CONTENT_TYPE, "application/octet-stream");
  add_header!(headers, header::ACCEPT_RANGES, "bytes");
  add_header!(headers, header::LAST_MODIFIED, HttpDate::from(meta.modified().unwrap()).to_string());
  response
}

#[inline]
fn apply_hash_headers(headers: &mut HeaderMap, map: FileMap) {
  if let Some(hash) = map.xxh3 {
    add_header!(headers, "X-Hash-Xxh3", hash);
  }
  if let Some(hash) = map.sha1 {
    add_header!(headers, "X-Hash-Sha1", hash);
  }
  if let Some(hash) = map.sha256 {
    add_header!(headers, "X-Hash-Sha256", hash);
  }
  if let Some(hash) = map.sha512 {
    add_header!(headers, "X-Hash-Sha512", hash);
  }
}
