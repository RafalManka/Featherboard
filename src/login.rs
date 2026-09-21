use crate::AppState;
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use sqlx::SqlitePool;
use tower_sessions::Session;

pub fn login_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_form))
        .route("/logout", get(logout))
}

#[derive(Template)]
#[template(path = "pages/admin_login.html")]
pub struct AdminLoginTemplate {
    layout: Layout,
    github_enabled: bool,
}

pub async fn is_logged_in(
    db: &SqlitePool,
    session: &Session,
    org_id: i64,
) -> Result<bool, AppError> {
    let user_id = session.get::<i64>("user_id").await?;
    let sql = r#"
        SELECT EXISTS (SELECT id FROM users WHERE id = ? AND org_id = ?)
    "#;

    let exists = sqlx::query_scalar(sql)
        .bind(user_id)
        .bind(org_id)
        .fetch_one(db)
        .await?;

    Ok(exists)
}

async fn login_form(
    State(state): State<AppState>,
    session: Session,
    current_org: CurrentOrg,
) -> Result<Response, AppError> {
    let github_enabled =
        current_org.github_client_secret.is_some() && current_org.github_client_id.is_some();

    Ok(HtmlTemplate(AdminLoginTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        github_enabled,
    })
    .into_response())
}

async fn logout(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    session.remove::<i64>("user_id").await?;
    Ok(current_org.redirect(String::new()).into_response())
}
