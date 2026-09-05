use crate::AppState;
use crate::error::AppError;
use crate::models::Comment;
use crate::org::CurrentOrg;
use crate::validation::{
    self, AUTHOR_NAME_MAX, AUTHOR_NAME_MIN, COMMENT_BODY_MAX, COMMENT_BODY_MIN,
};
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Form, Router};
use serde::Deserialize;
use std::collections::HashMap;

pub fn comments_router() -> Router<AppState> {
    Router::new().route("/{id}/comments", post(create_comment))
}

pub struct CommentWithReplies {
    pub comment: Comment,
    pub replies: Vec<Comment>,
}

/// Two-level threading only: replies to a reply are folded onto the
/// top-level comment they ultimately belong to (enforced at write time
/// in `create_comment`, so this is just partitioning what's already flat).
pub async fn comments_for_idea(
    state: &AppState,
    idea_id: i64,
) -> Result<Vec<CommentWithReplies>, AppError> {
    let all: Vec<Comment> = sqlx::query_as(
        "SELECT id, idea_id, parent_comment_id, author_name, body, created_at \
         FROM comments WHERE idea_id = ? ORDER BY created_at",
    )
    .bind(idea_id)
    .fetch_all(&state.db)
    .await?;

    let mut replies_by_parent: HashMap<i64, Vec<Comment>> = HashMap::new();
    let mut top_level = Vec::new();
    for comment in all {
        match comment.parent_comment_id {
            Some(parent_id) => replies_by_parent
                .entry(parent_id)
                .or_default()
                .push(comment),
            None => top_level.push(comment),
        }
    }

    Ok(top_level
        .into_iter()
        .map(|comment| {
            let replies = replies_by_parent.remove(&comment.id).unwrap_or_default();
            CommentWithReplies { comment, replies }
        })
        .collect())
}

#[derive(Deserialize)]
pub struct CreateCommentForm {
    author_name: String,
    body: String,
    parent_comment_id: Option<i64>,
}
#[derive(Deserialize)]
struct IdParam {
    id: i64,
}
async fn create_comment(
    State(state): State<AppState>,
    Path(id): Path<IdParam>,
    current_org: CurrentOrg,
    Form(form): Form<CreateCommentForm>,
) -> Result<Response, AppError> {
    let author_name = form.author_name.trim().to_string();
    let body = form.body.trim().to_string();

    let author_error =
        validation::validate_len(&author_name, "Name", AUTHOR_NAME_MIN, AUTHOR_NAME_MAX).err();
    let body_error =
        validation::validate_len(&body, "Comment", COMMENT_BODY_MIN, COMMENT_BODY_MAX).err();

    if author_error.is_some() || body_error.is_some() {
        return Ok(current_org
            .redirect(format!("/ideas/{}?comment_error=1", id.id))
            .into_response());
    }

    // Enforce 2-level threading: a reply's parent must belong to this idea and
    // must itself be top-level. Anything else (bad id, reply-to-a-reply) is
    // silently flattened to a top-level comment rather than rejected outright.
    let parent_comment_id = match form.parent_comment_id {
        Some(parent_id) => {
            let is_valid_parent: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM comments WHERE id = ? AND idea_id = ? AND parent_comment_id IS NULL)",
            )
            .bind(parent_id)
            .bind(id.id)
            .fetch_one(&state.db)
            .await?;
            is_valid_parent.then_some(parent_id)
        }
        None => None,
    };

    sqlx::query(
        "INSERT INTO comments (idea_id, parent_comment_id, author_name, body) VALUES (?, ?, ?, ?)",
    )
    .bind(id.id)
    .bind(parent_comment_id)
    .bind(&author_name)
    .bind(&body)
    .execute(&state.db)
    .await?;

    Ok(current_org
        .redirect(format!("/ideas/{}", id.id))
        .into_response())
}
