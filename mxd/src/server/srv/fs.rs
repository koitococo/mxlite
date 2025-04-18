use std::fs::{metadata, read_dir, symlink_metadata};

use anyhow::Result;
use axum::{
  Json, Router,
  body::Body,
  extract::Query,
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::get,
};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use super::super::utils::auth_middleware;
use crate::states::SharedAppState;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  let router = Router::new().with_state(app.clone()).route("/lsdir", get(get_lsdir)).route("/read", get(get_read));
  auth_middleware(router, app.startup_args.apikey.clone())
}

#[derive(Deserialize)]
struct GetLsdirParams {
  path: String,
}

#[derive(Serialize)]
struct GetLsdirResponse {
  ok: bool,
  error: Option<String>,
  existed: bool,
  result: Option<LsdirResult>,
}

async fn get_lsdir(Query(params): Query<GetLsdirParams>) -> Json<GetLsdirResponse> {
  debug!("retrieve dir info: {:?}", params.path);
  if !std::fs::exists(&params.path).unwrap_or(false) {
    return Json(GetLsdirResponse {
      ok: false,
      error: Some("Path does not exist".to_string()),
      existed: false,
      result: None,
    });
  }
  match lsdir(&params.path) {
    Ok(result) => Json(GetLsdirResponse {
      ok: true,
      error: None,
      existed: true,
      result: Some(result),
    }),
    Err(err) => Json(GetLsdirResponse {
      ok: false,
      error: Some(err.to_string()),
      existed: true,
      result: None,
    }),
  }
}

#[derive(Serialize)]
struct LsdirResult {
  files: Vec<String>,
  subdirs: Vec<String>,
  is_file: bool,
  is_symlink: bool,
  size: u64,
}

fn lsdir(path: &String) -> Result<LsdirResult> {
  let mut files = vec![];
  let mut subdirs = vec![];
  let mut meta = symlink_metadata(path)?;
  let is_symlink = meta.is_symlink();
  debug!(
    "lsdir: {:?}, is_symlink: {}, is_dir: {}, is_file: {}",
    path,
    is_symlink,
    meta.is_dir(),
    meta.is_file()
  );
  if is_symlink {
    meta = metadata(path)?;
  }
  if meta.is_dir() {
    if let Ok(entries) = read_dir(path) {
      for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
          let ft = entry.file_type().inspect_err(|e| {
            warn!("Failed to get file type: {:?}; path: {:?}", e, entry.path());
          })?;
          if ft.is_dir() {
            subdirs.push(name.to_string());
          } else if ft.is_file() {
            files.push(name.to_string());
          } else if ft.is_symlink() {
            match metadata(entry.path()) {
              Ok(target) => {
                if target.is_dir() {
                  subdirs.push(name.to_string());
                } else if target.is_file() {
                  files.push(name.to_string());
                } else {
                  warn!("Unknown file type: {:?}", entry.file_type());
                }
              }
              Err(e) => {
                warn!("Failed to get symlink target: {:?}; path: {:?}", e, entry.path());
              }
            }
          }
        }
      }
    }
    Ok(LsdirResult {
      files,
      subdirs,
      is_file: false,
      is_symlink,
      size: meta.len(),
    })
  } else {
    Ok(LsdirResult {
      files,
      subdirs,
      is_file: true,
      is_symlink,
      size: meta.len(),
    })
  }
}

#[derive(Deserialize)]
struct GetReadParams {
  path: String,
  max_size: Option<u64>,
}

async fn get_read(Query(params): Query<GetReadParams>) -> Response {
  debug!("get_read: {:?}", params.path);
  if let Some(size) = metadata(&params.path).ok().and_then(|meta| {
    if meta.is_file() {
      Some(meta.len())
    } else {
      debug!("get_read: Path is not a file: {:?}", params.path);
      None
    }
  }) {
    if params.max_size.map(|s| s >= size).unwrap_or(true) {
      tokio::fs::File::open(&params.path)
        .await
        .map(|file| Body::from_stream(ReaderStream::new(file)).into_response())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
      debug!("get_read: File size exceeds max size: {:?}", params.path);
      StatusCode::IM_A_TEAPOT.into_response()
    }
  } else {
    StatusCode::NOT_FOUND.into_response()
  }
}
