use crate::admin::is_admin;
use crate::email::{ChangelogCreatedEmail, notify_changelog_created};
use crate::error::AppError;
use crate::models::Changelog;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use crate::validation::{DESCRIPTION_MAX, DESCRIPTION_MIN, TITLE_MAX, TITLE_MIN};
use crate::{AppState, validation};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use tower_sessions::Session;

pub fn changelog_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_changelog))
        .route("/", get(list_changelogs))
        .route("/{id}", get(changelog_detail))
        .route("/new", get(new_changelog_form))
}

async fn new_changelog_form(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok(HtmlTemplate(ChangelogFormTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
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
#[template(path = "pages/changelog_form.html")]
struct ChangelogFormTemplate {
    layout: Layout,
    title: String,
    description: String,
    title_error: Option<String>,
    description_error: Option<String>,
}

async fn create_changelog(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<CreateChangelogForm>,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db, current_org.id).await? {
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
                layout: Layout::load(&state.db,&current_org, &session).await?,
                title,
                description,
                title_error,
                description_error,
            }),
        )
            .into_response());
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO changelogs (org_id, title, description) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(current_org.id)
    .bind(&title)
    .bind(&description)
    .fetch_one(&state.db)
    .await?;

    let path = format!("/changelogs/{}", id);

    if let Some(email_client) = state.email_client {
        let changelog_title = title;
        let changelog_body = description;
        let changelog_url = current_org.path(path.clone());
        let changelog_url = format!("{}{}", email_client.public_url, changelog_url);

        notify_changelog_created(
            &state.db,
            &current_org,
            &email_client,
            ChangelogCreatedEmail {
                changelog_title,
                changelog_body,
                changelog_url,
            },
        )
        .await?;
    }

    Ok(current_org.redirect(path).into_response())
}
#[derive(Template)]
#[template(path = "pages/changelog_list.html")]
struct ChangelogListTemplate {
    layout: Layout,
    is_admin: bool,
    changelogs: Vec<Changelog>,
}

async fn list_changelogs(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    let is_admin = is_admin(&session, &state.db, current_org.id).await?;

    let sql = r#"
        SELECT id, org_id, title, description, created_at
        FROM changelogs
        WHERE org_id = ?
        ORDER BY created_at DESC, id DESC
    "#;

    let changelogs = sqlx::query_as::<_, Changelog>(sql)
        .bind(current_org.id)
        .fetch_all(&state.db)
        .await?;

    Ok(HtmlTemplate(ChangelogListTemplate {
        layout: Layout::load(&state.db,&current_org, &session).await?,
        is_admin,
        changelogs,
    })
    .into_response())
}

#[derive(Template)]
#[template(path = "pages/changelog_detail.html")]
struct ChangelogDetailTemplate {
    layout: Layout,
    changelog: Changelog,
}

#[derive(Deserialize)]
struct IdParam {
    id: i64,
}

async fn changelog_detail(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Path(id): Path<IdParam>,
) -> Result<Response, AppError> {
    let sql = r#"
        SELECT id, org_id, title, description, created_at
        FROM changelogs
        WHERE org_id = ? AND id = ?
    "#;

    let changelog = sqlx::query_as::<_, Changelog>(sql)
        .bind(current_org.id)
        .bind(id.id)
        .fetch_optional(&state.db)
        .await?;

    let Some(changelog) = changelog else {
        return Ok((StatusCode::NOT_FOUND, "changelog not found").into_response());
    };

    Ok(HtmlTemplate(ChangelogDetailTemplate {
        layout: Layout::load(&state.db,&current_org, &session).await?,
        changelog,
    })
    .into_response())
}
