use crate::AppState;
use crate::error::AppError;
use crate::models::IdeaStatus;
use crate::org::CurrentOrg;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post};
use axum::{Form, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;

/// Merged into the `/ideas` router by `ideas::ideas_router`.
pub fn idea_admin_router() -> Router<AppState> {
    Router::new()
        .route("/{id}/status", post(update_status))
        .route("/{id}/changelog", post(assign_changelog))
        .route("/{id}", delete(delete_idea))
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

#[derive(Deserialize)]
struct IdParam {
    id: i64,
}
async fn update_status(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Path(id_path): Path<IdParam>,
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
        .bind(id_path.id)
        .execute(&state.db)
        .await?;
    Ok(current_org
        .redirect(format!("/ideas/{}", id_path.id))
        .into_response())
}

#[derive(Deserialize)]
pub struct AssignChangelogForm {
    changelog_id: String,
}

async fn assign_changelog(
    State(state): State<AppState>,
    Path(id_path): Path<IdParam>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<AssignChangelogForm>,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let changelog_id: Option<i64> = if form.changelog_id.is_empty() {
        None
    } else {
        let Ok(id) = form.changelog_id.parse::<i64>() else {
            return Ok(StatusCode::BAD_REQUEST.into_response());
        };
        Some(id)
    };

    let sql = r#"
        UPDATE ideas
        SET changelog_id = ?
        WHERE id = ? AND org_id = ?
    "#;

    sqlx::query(sql)
        .bind(changelog_id)
        .bind(id_path.id)
        .bind(current_org.id)
        .execute(&state.db)
        .await?;

    Ok(current_org
        .redirect(format!("/ideas/{}", id_path.id))
        .into_response())
}

async fn delete_idea(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Path(id_path): Path<IdParam>,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    sqlx::query("DELETE FROM ideas WHERE id = ? AND org_id = ?")
        .bind(id_path.id)
        .bind(current_org.id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::OK, current_org.hx_redirect(String::new())).into_response())
}
