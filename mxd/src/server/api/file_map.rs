use axum::{
  Json, Router,
  extract::{Query, State},
  http::StatusCode,
  routing::method_routing,
};
use serde::{Deserialize, Serialize};

use crate::states::SharedAppState;

#[derive(Deserialize)]
struct PostRequestMapInner {
  path: String,
  name: String,
  isdir: Option<bool>,
}

#[derive(Deserialize)]
struct PostRequest {
  maps: Vec<PostRequestMapInner>,
}

#[derive(Serialize)]
struct PostResponseErrInner {
  ok: bool,
  err: Option<String>,
  name: String,
}

#[derive(Serialize)]
struct PostResponse {
  result: Vec<PostResponseErrInner>,
}

async fn post(State(app): State<SharedAppState>, Json(params): Json<PostRequest>) -> Json<PostResponse> {
  let mut result = Vec::with_capacity(params.maps.len());
  for map in params.maps {
    if map.isdir.unwrap_or(false) {
      if let Err(e) = app.file_map.add_dir_map(map.path, map.name.clone()) {
        result.push(PostResponseErrInner {
          ok: false,
          err: Some(e),
          name: map.name,
        });
      } else {
        result.push(PostResponseErrInner {
          ok: true,
          err: None,
          name: map.name,
        });
      }
    } else if let Err(e) = app.file_map.add_file_map(map.path, map.name.clone()) {
      result.push(PostResponseErrInner {
        ok: false,
        err: Some(e),
        name: map.name,
      });
    } else {
      result.push(PostResponseErrInner {
        ok: true,
        err: None,
        name: map.name,
      });
    }
  }
  Json(PostResponse { result })
}

#[derive(Serialize)]
struct GetResponse {
  files: Vec<String>,
}

async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
  Json(GetResponse {
    files: app.file_map.list_map(),
  })
}

#[derive(Deserialize)]
struct DeleteRequest {
  publish_name: String,
}

async fn delete(State(app): State<SharedAppState>, Query(params): Query<DeleteRequest>) -> StatusCode {
  app.file_map.del_map(&params.publish_name);
  StatusCode::OK
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app.clone()).route("/", method_routing::get(get).post(post).delete(delete))
}
