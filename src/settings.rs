use crate::AppState;
use crate::site_settings::site_settings_router;
use crate::user_settings::user_settings_router;
use axum::Router;

pub fn settings_router() -> Router<AppState> {
    Router::new()
        .nest("/site", site_settings_router())
        .nest("/user", user_settings_router())
}
