use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::states::SharedAppState;

#[derive(Deserialize)]
pub(super) struct GetParams {
    host: String,
}

pub(super) async fn get(
    State(app): State<SharedAppState>,
    params: Query<GetParams>,
) -> Json<Vec<u64>> {
    let tasks = app.host_session.list_all_tasks(&params.host).await;
    Json(tasks)
}
