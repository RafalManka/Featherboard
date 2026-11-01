use crate::AppState;
use crate::auth_github::github_router;
use axum::Router;
use tower_sessions::Session;
use tower_sessions::session::Error;

pub fn auth_router() -> Router<AppState> {
    Router::new().nest("/github", github_router())
}

pub async fn get_user_id(session: &Session) -> Result<Option<i64>, Error> {
    Ok(session.get::<i64>("user_id").await?)
}
