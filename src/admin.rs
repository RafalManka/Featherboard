use crate::AppState;
use crate::error::AppError;
use crate::models::IdeaStatus;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::post;
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;

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
