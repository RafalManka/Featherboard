use crate::AppState;
use crate::error::AppError;
use crate::models::IdeaStatus;
use crate::templates::HtmlTemplate;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(admin_login))
        .route("/login", get(admin_login_form))
}

/// Merged into the `/ideas` router by `ideas::ideas_router`.
pub fn idea_status_router() -> Router<AppState> {
    Router::new().route("/{id}/status", post(update_status))
}

pub async fn is_admin(session: &Session, db: &SqlitePool) -> Result<bool, AppError> {
    let Some(user_id) = session.get::<i64>("user_id").await? else {
        return Ok(false);
    };

    let sql = r#"
        SELECT is_admin
        FROM users
        WHERE id = ?
    "#;

    let is_admin = sqlx::query_scalar(sql)
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .unwrap_or(false);

    Ok(is_admin)
}

#[derive(Template)]
#[template(path = "admin_login.html")]
pub struct AdminLoginTemplate {
    token: String,
    token_error: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminLoginForm {
    token: String,
}

async fn admin_login_form() -> HtmlTemplate<AdminLoginTemplate> {
    HtmlTemplate(AdminLoginTemplate {
        token: "".to_string(),
        token_error: None,
    })
}

/// Interim stopgap until Phase 4 adds real admin auth: a single shared
/// secret from the environment. If it's unset, admin login is disabled
/// entirely (safe-by-default) rather than falling open.
async fn admin_login(
    session: Session,
    Form(form): Form<AdminLoginForm>,
) -> Result<Response, AppError> {
    let Ok(expected) = std::env::var("FEATHERBOARD_ADMIN_TOKEN") else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            HtmlTemplate(AdminLoginTemplate {
                token: form.token,
                token_error: Some("Admin account not set".to_string()),
            }),
        )
            .into_response());
    };

    if form.token != expected {
        return Ok((
            StatusCode::UNAUTHORIZED,
            HtmlTemplate(AdminLoginTemplate {
                token: form.token,
                token_error: Some("Token incorrect".to_string()),
            }),
        )
            .into_response());
    }

    session.insert("is_admin", true).await?;
    Ok(Redirect::to("/").into_response())
}

#[derive(Deserialize)]
pub struct UpdateStatusForm {
    status: String,
}

async fn update_status(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<UpdateStatusForm>,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    let Some(status) = IdeaStatus::parse(&form.status) else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    sqlx::query("UPDATE ideas SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Redirect::to(&format!("/ideas/{id}")).into_response())
}
