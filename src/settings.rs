use crate::AppState;
use crate::admin::is_admin;
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::subscription::{SubscriptionView, get_subscription_view};
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower_sessions::Session;

pub fn settings_router() -> Router<AppState> {
    Router::new().route("/", get(get_settings))
}

#[derive(Template)]
#[template(path = "pages/settings.html")]
struct SettingsTemplate {
    layout: Layout,
    subscription: SubscriptionView,
}

async fn get_settings(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_admin(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok(HtmlTemplate(SettingsTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        subscription: get_subscription_view(&state.db, current_org.id).await?,
    })
    .into_response())
}
