use axum::{
  Router,
  extract::{Request, State},
  http::StatusCode,
  middleware::{self, Next},
  response::IntoResponse as _,
};

#[derive(Clone)]
struct ApiState {
  apikey: Option<String>,
}

pub(super) fn auth_middleware<T: Clone + Send + Sync + 'static>(router: Router<T>, key: Option<String>) -> Router<T> {
  router.layer(middleware::from_fn_with_state(
    ApiState {
      apikey: key.map(|sk| format!("Bearer {}", sk)),
    },
    async |State(state): State<ApiState>, request: Request, next: Next| {
      if let Some(sk) = state.apikey {
        if let Some(key) = request.headers().get("Authorization") {
          if key != &sk {
            return (StatusCode::FORBIDDEN).into_response();
          }
        } else {
          return (StatusCode::UNAUTHORIZED).into_response();
        }
      }
      next.run(request).await
    },
  ))
}
