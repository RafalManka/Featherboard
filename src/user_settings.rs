use crate::AppState;
use crate::error::AppError;
use crate::login::is_logged_in;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use tower_sessions::Session;

pub fn user_settings_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_user_settings))
        .route("/notifications", post(set_notifications))
}

#[derive(Template)]
#[template(path = "pages/user_settings.html")]
struct UserSettingsTemplate {
    layout: Layout,
    notify_comments: bool,
    notify_status_changes: bool,
    notify_shipped: bool,
}

async fn get_user_settings(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_logged_in(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let Some(user_id) = session.get::<i64>("user_id").await? else {
        return Ok(StatusCode::FORBIDDEN.into_response());
    };

    let preferences: (bool, bool, bool) = sqlx::query_as(
        "SELECT notify_comments, notify_status_changes, notify_shipped
         FROM users
         WHERE id = ? AND org_id = ?",
    )
    .bind(user_id)
    .bind(current_org.id)
    .fetch_one(&state.db)
    .await?;

    Ok(HtmlTemplate(UserSettingsTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        notify_comments: preferences.0,
        notify_status_changes: preferences.1,
        notify_shipped: preferences.2,
    })
    .into_response())
}

#[derive(Deserialize)]
struct NotificationsForm {
    notify_comments: Option<String>,
    notify_status_changes: Option<String>,
    notify_shipped: Option<String>,
}

async fn set_notifications(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<NotificationsForm>,
) -> Result<Response, AppError> {
    if !is_logged_in(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let Some(user_id) = session.get::<i64>("user_id").await? else {
        return Ok(StatusCode::FORBIDDEN.into_response());
    };

    sqlx::query(
        "UPDATE users
         SET notify_comments = ?, notify_status_changes = ?, notify_shipped = ?
         WHERE id = ? AND org_id = ?",
    )
    .bind(form.notify_comments.is_some())
    .bind(form.notify_status_changes.is_some())
    .bind(form.notify_shipped.is_some())
    .bind(user_id)
    .bind(current_org.id)
    .execute(&state.db)
    .await?;

    Ok(current_org.redirect("/settings/user").into_response())
}
