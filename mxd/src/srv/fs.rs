use anyhow::Result;
use axum::{
    Json, Router,
    extract::Query,
    routing::get,
};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{api::auth_middleware, states::SharedAppState};

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
    let router = Router::new()
        .with_state(app.clone())
        .route("/lsdir", get(get_lsdir));
    auth_middleware(router, app.startup_args.apikey.clone())
}

#[derive(Deserialize)]
struct GetFsParams {
    path: String,
}

#[derive(Serialize)]
struct GetLsdirResponse {
    ok: bool,
    error: Option<String>,
    existed: bool,
    result: Option<LsdirResult>,
}

async fn get_lsdir(Query(params): Query<GetFsParams>) -> Json<GetLsdirResponse> {
    debug!("lsdir: {:?}", params.path);
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
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_symlink() {
        let meta = std::fs::metadata(path)?;
        if meta.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        let ft = entry.file_type()?;
                        if ft.is_dir() {
                            subdirs.push(name.to_string());
                        } else if ft.is_file() {
                            files.push(name.to_string());
                        } else if ft.is_symlink() {
                            let target = std::fs::read_link(entry.path())?;
                            if target.is_dir() {
                                subdirs.push(name.to_string());
                            } else if target.is_file() {
                                files.push(name.to_string());
                            }
                        }
                    }
                }
            }
            Ok(LsdirResult {
                files,
                subdirs,
                is_file: false,
                is_symlink: true,
                size: meta.len(),
            })
        } else {
            Ok(LsdirResult {
                files,
                subdirs,
                is_file: true,
                is_symlink: true,
                size: meta.len(),
            })
        }
    } else {
        if meta.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        let ft = entry.file_type()?;
                        if ft.is_dir() {
                            subdirs.push(name.to_string());
                        } else if ft.is_file() {
                            files.push(name.to_string());
                        } else if ft.is_symlink() {
                            let target = std::fs::read_link(entry.path())?;
                            if target.is_dir() {
                                subdirs.push(name.to_string());
                            } else if target.is_file() {
                                files.push(name.to_string());
                            }
                        }
                    }
                }
            }
            Ok(LsdirResult {
                files,
                subdirs,
                is_file: false,
                is_symlink: false,
                size: meta.len(),
            })
        } else {
            Ok(LsdirResult {
                files,
                subdirs,
                is_file: true,
                is_symlink: false,
                size: meta.len(),
            })
        }
    }
}
