use axum::Router;

use crate::states::SharedAppState;

mod file;
mod fs;
mod url_sub;

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  axum::Router::new()
    .nest("/file", file::build(app.clone()))
    .nest("/url-sub", url_sub::build(app.clone()))
    .nest("/fs", fs::build(app.clone()))
}
