use crate::AppState;
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::Router;
use axum::response::{IntoResponse, Response};
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
    layout: Layout,
    github_enabled: bool,
}

pub async fn is_logged_in(session: &Session) -> Result<bool, AppError> {
    Ok(session.get::<i64>("user_id").await?.is_some())
}

async fn login_form(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    let github_enabled =
        current_org.github_client_secret.is_some() && current_org.github_client_id.is_some();

    Ok(HtmlTemplate(AdminLoginTemplate {
        layout: Layout::load(&current_org, &session).await?,
        github_enabled,
    })
    .into_response())
}

async fn logout(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    session.remove::<i64>("user_id").await?;
    Ok(current_org.redirect(String::new()).into_response())
}
