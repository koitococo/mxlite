use axum::{
    Router,
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::states::SharedAppState;
use axum::extract::Path;

pub(crate) fn build(app: SharedAppState) -> Router<SharedAppState> {
    Router::new()
        .with_state(app.clone())
        .route("/file/{name}", get(get_file))
}

async fn get_file(State(app): State<SharedAppState>, Path(name): Path<String>) -> Response {
    if name.ends_with(".sha1") {
        let hash = name.trim_end_matches(".sha1");
        if hash.len() != 40 {
            (StatusCode::BAD_REQUEST, "Invalid hash length".to_string()).into_response()
        } else {
            (StatusCode::OK, hash.to_string()).into_response()
        }
    } else if let Some(path) = app.get_file(&name).await {
        let fd = File::open(path).await;
        match fd {
            Ok(file) => {
                (StatusCode::OK, Body::from_stream(ReaderStream::new(file))).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error opening file: {}", e),
            )
                .into_response(),
        }
    } else {
        (StatusCode::NOT_FOUND, "File not found".to_string()).into_response()
    }
}
