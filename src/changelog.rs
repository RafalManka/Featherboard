use crate::admin::is_admin;
use crate::error::AppError;
use crate::models::Changelog;
use crate::templates::HtmlTemplate;
use crate::validation::{DESCRIPTION_MAX, DESCRIPTION_MIN, TITLE_MAX, TITLE_MIN};
use crate::{AppState, validation};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use tower_sessions::Session;

pub fn changelog_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_changelog))
        .route("/", get(list_changelogs))
        .route("/new", get(new_changelog_form))
}

async fn new_changelog_form(session: Session) -> Result<Response, AppError> {
    if !is_admin(&session).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok(HtmlTemplate(ChangelogFormTemplate {
        title: String::new(),
        description: String::new(),
        title_error: None,
        description_error: None,
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct CreateChangelogForm {
    title: String,
    description: String,
}

#[derive(Template)]
#[template(path = "changelog_form.html")]
struct ChangelogFormTemplate {
    title: String,
    description: String,
    title_error: Option<String>,
    description_error: Option<String>,
}

async fn create_changelog(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CreateChangelogForm>,
) -> Result<Response, AppError> {
    if !is_admin(&session).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let title = form.title.trim().to_string();
    let description = form.description.trim().to_string();

    let title_error = validation::validate_len(&title, "Title", TITLE_MIN, TITLE_MAX).err();
    let description_error = validation::validate_len(
        &description,
        "Description",
        DESCRIPTION_MIN,
        DESCRIPTION_MAX,
    )
    .err();

    if title_error.is_some() || description_error.is_some() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            HtmlTemplate(ChangelogFormTemplate {
                title,
                description,
                title_error,
                description_error,
            }),
        )
            .into_response());
    }

    sqlx::query("INSERT INTO changelogs (org_id, title, description) VALUES (?, ?, ?)")
        .bind(state.default_org_id)
        .bind(&title)
        .bind(&description)
        .execute(&state.db)
        .await?;

    Ok(Redirect::to("/changelogs").into_response())
}
#[derive(Template)]
#[template(path = "changelog_list.html")]
struct ChangelogListTemplate {
    is_admin: bool,
    changelogs: Vec<Changelog>,
}

async fn list_changelogs(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    let is_admin = is_admin(&session).await?;

    let sql = r#"
        SELECT id, org_id, title, description, created_at
        FROM changelogs
        WHERE org_id = ?
        ORDER BY created_at DESC, id DESC
    "#;

    let changelogs = sqlx::query_as::<_, Changelog>(sql)
        .bind(state.default_org_id)
        .fetch_all(&state.db)
        .await?;

    Ok(HtmlTemplate(ChangelogListTemplate {
        is_admin,
        changelogs,
    })
    .into_response())
}
