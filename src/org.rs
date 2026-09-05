use crate::AppState;
use crate::error::AppError;
use axum::extract::{FromRequestParts, OriginalUri};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use std::convert::Infallible;

pub enum CurrentOrgRejection {
    NotFound,
    Error(AppError),
    Infallible(Infallible),
}

impl From<Infallible> for CurrentOrgRejection {
    fn from(value: Infallible) -> Self {
        CurrentOrgRejection::Infallible(value)
    }
}
impl IntoResponse for CurrentOrgRejection {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::Error(e) => e.into_response(),
            Self::Infallible(e) => e.into_response(),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct CurrentOrg {
    pub id: i64,
    pub slug: String,
    pub github_client_secret: Option<String>,
    pub github_client_id: Option<String>,
}

impl CurrentOrg {
    fn path(self, path: String) -> String {
        format!("/{}{}", self.slug, path)
    }
    pub fn redirect(self, p: String) -> Redirect {
        Redirect::to(self.path(p).as_str())
    }
    pub fn hx_redirect(self, p: String) -> [(&'static str, String); 1] {
        [("HX-Redirect", self.path(p))]
    }
}

impl FromRequestParts<AppState> for CurrentOrg {
    type Rejection = CurrentOrgRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let original_uri = OriginalUri::from_request_parts(parts, state).await?;
        let slug: String = original_uri.path().split('/').skip(1).take(1).collect();

        let sql = r#"
            SELECT id, slug, github_client_secret, github_client_id
            FROM organizations
            WHERE slug = ?
        "#;

        let current_org: Option<CurrentOrg> = sqlx::query_as::<_, CurrentOrg>(sql)
            .bind(slug)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| CurrentOrgRejection::Error(e.into()))?;

        match current_org {
            None => Err(CurrentOrgRejection::NotFound),
            Some(current_org) => Ok(current_org),
        }
    }
}
