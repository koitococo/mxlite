use axum::{Json, extract::State};
use futures_util::future::join_all;
use serde::Serialize;

use crate::states::{SharedAppState, host_session::ExtraInfo};

#[derive(Serialize)]
pub(super) struct GetRespInner {
    host: String,
    info: Option<ExtraInfo>,
}

#[derive(Serialize)]
pub(super) struct GetResponse {
    ok: bool,
    hosts: Vec<GetRespInner>,
}

pub(super) async fn get(State(app): State<SharedAppState>) -> Json<GetResponse> {
    let hosts = join_all(
        app.host_session
            .list_sessions()
            .await
            .iter()
            .map(async |s| GetRespInner {
                host: s.clone(),
                info: app.host_session.get_extra_info(s).await,
            }),
    )
    .await;
    Json(GetResponse { ok: true, hosts })
}
