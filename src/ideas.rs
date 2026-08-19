use crate::AppState;
use crate::admin::{self, idea_admin_router};
use crate::comments::{self, CommentWithReplies};
use crate::error::AppError;
use crate::login::is_logged_in;
use crate::models::{Changelog, Idea, IdeaStatus};
use crate::org::CurrentOrg;
use crate::templates::HtmlTemplate;
use crate::validation::{self, DESCRIPTION_MAX, DESCRIPTION_MIN, TITLE_MAX, TITLE_MIN};
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use std::collections::HashSet;
use tower_sessions::Session;

pub fn ideas_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_idea))
        .route("/new", get(new_idea_form))
        .route("/{id}", get(idea_detail))
        .route("/{id}/vote", post(vote))
        .merge(comments::comments_router())
        .merge(idea_admin_router())
}



#[derive(sqlx::FromRow)]
struct IdeaRow {
    id: i64,
    org_id: i64,
    title: String,
    description: String,
    status: String,
    created_at: String,
    vote_count: i64,
    changelog_id: Option<i64>,
}

impl IdeaRow {
    fn into_idea(self) -> Idea {
        Idea {
            id: self.id,
            org_id: self.org_id,
            title: self.title,
            description: self.description,
            status: self.status,
            created_at: self.created_at,
            changelog_id: self.changelog_id,
        }
    }
}

struct IdeaListItem {
    idea: Idea,
    vote_count: i64,
    voted: bool,
}

/// Session ids are only assigned once the session store has actually persisted
/// a record. Anonymous visitors who haven't voted yet may have no id yet, in
/// which case they can't have voted on anything either.
async fn voted_idea_ids(state: &AppState, session: &Session) -> Result<HashSet<i64>, AppError> {
    let Some(voter_id) = session.id() else {
        return Ok(HashSet::new());
    };
    let ids: Vec<i64> = sqlx::query_scalar("SELECT idea_id FROM votes WHERE voter_id = ?")
        .bind(voter_id.to_string())
        .fetch_all(&state.db)
        .await?;
    Ok(ids.into_iter().collect())
}

#[derive(Template)]
#[template(path = "idea_list.html")]
pub struct IdeaListTemplate {
    org_slug: String,
    is_logged_in: bool,
    ideas: Vec<IdeaListItem>,
    current_status: String,
    current_sort: String,
    all_statuses: [IdeaStatus; 5],
}

#[derive(Deserialize)]
pub struct IdeaListQuery {
    status: Option<String>,
    sort: Option<String>,
}

pub async fn list_ideas(
    current_org: CurrentOrg,
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<IdeaListQuery>,
) -> Result<HtmlTemplate<IdeaListTemplate>, AppError> {
    let status_filter: Option<IdeaStatus> = query.status.as_deref().and_then(IdeaStatus::parse);
    // Sort direction is picked via a Rust match over a fixed whitelist, never
    // string-interpolated from the raw query param directly: column/direction
    // can't be parameter-bound, so that would be a SQL-injection-shaped hole.
    let is_top_sort = query.sort.as_deref() == Some("top");
    let order_by = if is_top_sort {
        "vote_count DESC, i.id DESC"
    } else {
        "i.created_at DESC"
    };

    let mut sql = r#"
            SELECT
                i.id,
                i.org_id,
                i.title,
                i.description,
                i.status,
                i.created_at,
                i.changelog_id,
                COUNT(v.id) AS vote_count
            FROM ideas i
                LEFT JOIN votes v ON v.idea_id = i.id
            WHERE i.org_id = ?
        "#
    .to_string();
    if status_filter.is_some() {
        sql.push_str(" AND i.status = ?");
    }
    sql.push_str(&format!(" GROUP BY i.id ORDER BY {order_by}"));

    let mut q = sqlx::query_as::<_, IdeaRow>(&sql).bind(current_org.id);
    if let Some(status) = status_filter {
        q = q.bind(status.as_str());
    }
    let rows: Vec<IdeaRow> = q.fetch_all(&state.db).await?;

    let voted_ids = voted_idea_ids(&state, &session).await?;
    let ideas = rows
        .into_iter()
        .map(|row| {
            let voted = voted_ids.contains(&row.id);
            let vote_count = row.vote_count;
            IdeaListItem {
                idea: row.into_idea(),
                vote_count,
                voted,
            }
        })
        .collect();

    Ok(HtmlTemplate(IdeaListTemplate {
        org_slug: current_org.slug,
        is_logged_in: is_logged_in(&session).await?,
        ideas,
        current_status: status_filter
            .map(IdeaStatus::as_str)
            .unwrap_or("")
            .to_string(),
        current_sort: if is_top_sort { "top" } else { "new" }.to_string(),
        all_statuses: IdeaStatus::ALL,
    }))
}

#[derive(Template)]
#[template(path = "roadmap.html")]
pub struct RoadmapTemplate {
    org_slug: String,
    is_logged_in: bool,
    groups: Vec<(IdeaStatus, Vec<IdeaListItem>)>,
}

pub async fn roadmap(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<HtmlTemplate<RoadmapTemplate>, AppError> {
    let sql = r#"
        SELECT
            i.id,
            i.org_id,
            i.title,
            i.description,
            i.status,
            i.created_at,
            i.changelog_id,
            COUNT(v.id) AS vote_count
        FROM ideas i
            LEFT JOIN votes v ON v.idea_id = i.id
        WHERE i.org_id = ?
        GROUP BY i.id
        ORDER BY vote_count DESC, i.id DESC
    "#;

    let rows: Vec<IdeaRow> = sqlx::query_as(&sql)
        .bind(current_org.id)
        .fetch_all(&state.db)
        .await?;

    let voted_ids = voted_idea_ids(&state, &session).await?;
    let mut groups: Vec<(IdeaStatus, Vec<IdeaListItem>)> = IdeaStatus::ALL
        .into_iter()
        .map(|s| (s, Vec::new()))
        .collect();

    for row in rows {
        let Some(status) = IdeaStatus::parse(&row.status) else {
            continue;
        };
        let voted = voted_ids.contains(&row.id);
        let vote_count = row.vote_count;
        let item = IdeaListItem {
            idea: row.into_idea(),
            vote_count,
            voted,
        };
        if let Some((_, items)) = groups.iter_mut().find(|(s, _)| *s == status) {
            items.push(item);
        }
    }

    Ok(HtmlTemplate(RoadmapTemplate {
        org_slug: current_org.slug,
        is_logged_in: is_logged_in(&session).await?,
        groups,
    }))
}

#[derive(Template)]
#[template(path = "idea_detail.html")]
struct IdeaDetailTemplate {
    org_slug: String,
    is_logged_in: bool,
    item: IdeaListItem,
    comments: Vec<CommentWithReplies>,
    comment_error: bool,
    is_admin: bool,
    all_statuses: [IdeaStatus; 5],
    all_changelogs: Vec<Changelog>,
}

#[derive(Deserialize)]
pub struct IdeaDetailQuery {
    comment_error: Option<String>,
}

#[derive(Deserialize)]
struct IdParam {
    id: i64,
}

async fn idea_detail(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Path(id): Path<IdParam>,
    Query(query): Query<IdeaDetailQuery>,
) -> Result<Response, AppError> {
    let sql = r#"
        SELECT
            i.id,
            i.org_id,
            i.title,
            i.description,
            i.status,
            i.created_at,
            i.changelog_id,
            COUNT(v.id) AS vote_count
        FROM ideas i
            LEFT JOIN votes v ON v.idea_id = i.id
        WHERE i.id = ? AND i.org_id = ?
        GROUP BY i.id
    "#;
    let row: Option<IdeaRow> = sqlx::query_as(&sql)
        .bind(id.id)
        .bind(current_org.id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Ok((StatusCode::NOT_FOUND, "idea not found").into_response());
    };

    let voted = voted_idea_ids(&state, &session).await?.contains(&row.id);
    let vote_count = row.vote_count;
    let item = IdeaListItem {
        idea: row.into_idea(),
        vote_count,
        voted,
    };
    let comments = comments::comments_for_idea(&state, id.id).await?;
    let is_admin = admin::is_admin(&session, &state.db).await?;

    let sql = r#"
        SELECT
            id,
            org_id,
            title,
            description,
            created_at
        FROM changelogs
        WHERE org_id = ?
        ORDER BY title ASC;
    "#;

    let all_changelogs: Vec<Changelog> = sqlx::query_as(&sql)
        .bind(current_org.id)
        .fetch_all(&state.db)
        .await?;

    let comment_error = query.comment_error.is_some();
    let all_statuses = IdeaStatus::ALL;
    let is_logged_in = is_logged_in(&session).await?;
    let org_slug = current_org.slug;

    Ok(HtmlTemplate(IdeaDetailTemplate {
        org_slug,
        is_logged_in,
        item,
        comments,
        is_admin,
        all_statuses,
        comment_error,
        all_changelogs,
    })
    .into_response())
}

#[derive(Template)]
#[template(path = "partials/vote_button.html")]
struct VoteButtonTemplate {
    item: IdeaListItem,
}

async fn vote(
    State(state): State<AppState>,
    session: Session,
    current_org: CurrentOrg,
    Path(id): Path<IdParam>,
) -> Result<Response, AppError> {
    if session.id().is_none() {
        // Writing data marks the session modified, which is what makes the
        // tower-sessions middleware actually emit a Set-Cookie header for it
        // once the handler returns (an id with no modification is invisible
        // to the client). Saving immediately (rather than waiting for the
        // middleware to do it after we return) is what makes the id
        // available synchronously, right here, for the INSERT below.
        session.insert("has_voted", true).await?;
        session.save().await?;
    }
    let voter_id = session
        .id()
        .expect("session id is assigned immediately after save()")
        .to_string();

    let already_voted: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM votes WHERE idea_id = ? AND voter_id = ?)")
            .bind(id.id)
            .bind(&voter_id)
            .fetch_one(&state.db)
            .await?;

    let voted = if already_voted {
        sqlx::query("DELETE FROM votes WHERE idea_id = ? AND voter_id = ?")
            .bind(id.id)
            .bind(&voter_id)
            .execute(&state.db)
            .await?;
        false
    } else {
        sqlx::query("INSERT INTO votes (idea_id, voter_id) VALUES (?, ?)")
            .bind(id.id)
            .bind(&voter_id)
            .execute(&state.db)
            .await?;
        true
    };

    let sql = r#"
      SELECT
            i.id,
            i.org_id,
            i.title,
            i.description,
            i.status,
            i.created_at,
            i.changelog_id,
            COUNT(v.id) AS vote_count
      FROM ideas i
            LEFT JOIN votes v ON v.idea_id = i.id
      WHERE i.id = ? and i.org_id = ?
      GROUP BY i.id
    "#;

    let row: Option<IdeaRow> = sqlx::query_as(&sql)
        .bind(id.id)
        .bind(current_org.id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Ok((StatusCode::NOT_FOUND, "idea not found").into_response());
    };

    let vote_count = row.vote_count;
    let item = IdeaListItem {
        idea: row.into_idea(),
        vote_count,
        voted,
    };

    Ok(HtmlTemplate(VoteButtonTemplate { item }).into_response())
}

#[derive(Template)]
#[template(path = "idea_form.html")]
struct IdeaFormTemplate {
    org_slug: String,
    is_logged_in: bool,
    title: String,
    description: String,
    title_error: Option<String>,
    description_error: Option<String>,
}

async fn new_idea_form(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    Ok(HtmlTemplate(IdeaFormTemplate {
        org_slug: current_org.slug,
        is_logged_in: is_logged_in(&session).await?,
        title: String::new(),
        description: String::new(),
        title_error: None,
        description_error: None,
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct CreateIdeaForm {
    title: String,
    description: String,
}

async fn create_idea(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<CreateIdeaForm>,
) -> Result<Response, AppError> {
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
            HtmlTemplate(IdeaFormTemplate {
                org_slug: current_org.slug,
                is_logged_in: is_logged_in(&session).await?,
                title,
                description,
                title_error,
                description_error,
            }),
        )
            .into_response());
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO ideas (org_id, title, description) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(current_org.id)
    .bind(&title)
    .bind(&description)
    .fetch_one(&state.db)
    .await?;

    Ok(Redirect::to(&current_org.path(format!("/ideas/{id}"))).into_response())
}
