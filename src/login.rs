use crate::AppState;
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::templates::HtmlTemplate;
use askama::Template;
use axum::Router;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use tower_sessions::Session;

pub fn login_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_form))
        .route("/logout", get(logout))
}

#[derive(Template)]
#[template(path = "admin_login.html")]
pub struct AdminLoginTemplate {
    org_slug: String,
    is_logged_in: bool,
    github_enabled: bool,
}

pub async fn is_logged_in(session: &Session) -> Result<bool, AppError> {
    Ok(session.get::<i64>("user_id").await?.is_some())
}

async fn login_form(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    let github_enabled = std::env::var("GITHUB_CLIENT_ID").ok().is_some()
        && std::env::var("GITHUB_CLIENT_SECRET").ok().is_some();

    Ok(HtmlTemplate(AdminLoginTemplate {
        org_slug: current_org.slug,
        is_logged_in: is_logged_in(&session).await?,
        github_enabled,
    })
    .into_response())
}

async fn logout(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    session.remove::<i64>("user_id").await?;
    Ok(Redirect::to(&current_org.path(String::new())).into_response())
}
