use crate::AppState;
use crate::auth_github::github_router;
use axum::Router;

pub fn auth_router() -> Router<AppState> {
    Router::new().nest("/github", github_router())
}
