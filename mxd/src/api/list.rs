use axum::{Json, extract::State};
use serde::Serialize;

use crate::states::SharedAppState;

#[derive(Serialize)]
pub(super) struct GetResponse {
    ok: bool,
    sessions: Vec<String>,
}

pub(super) async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
    Json(GetResponse { ok: true, sessions: app.host_session.list() })
}
