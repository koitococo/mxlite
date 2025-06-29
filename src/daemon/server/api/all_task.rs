use axum::{
  Json, Router,
  extract::{Query, State},
  routing::method_routing,
};
use serde::Deserialize;

use crate::{daemon::states::SharedAppState, utils::states::States as _};

#[derive(Deserialize)]
struct GetParams {
  host: String,
}

async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> Json<Vec<u32>> {
  let Some(session) = app.host_session.get_arc(&params.host) else {
    return Json(vec![]);
  };
  Json(session.tasks.list())
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> {
  Router::new().with_state(app).route("/", method_routing::get(get))
}
