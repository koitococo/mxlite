use axum::{
  Json, Router,
  extract::{Query, State},
  routing::method_routing,
};
use serde::Deserialize;

use crate::states::SharedAppState;

#[derive(Deserialize)]
struct GetParams {
  host: String,
}

async fn get(State(app): State<SharedAppState>, params: Query<GetParams>) -> Json<Vec<u64>> {
  let tasks = app.host_session.list_all_tasks(&params.host).await;
  Json(tasks)
}

pub(super) fn build(app: SharedAppState) -> Router<SharedAppState> { Router::new().with_state(app.clone()).route("/", method_routing::get(get)) }
