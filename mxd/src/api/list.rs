use axum::{Json, extract::State};
use serde::Serialize;

use crate::states::SharedAppState;

#[derive(Serialize)]
pub(super) struct GetResponse {
    ok: bool,
    sessions: Vec<String>,
}

pub(super) async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
    let sessions = app.host_session.list_sessions().await;
    Json(GetResponse { ok: true, sessions })
}
