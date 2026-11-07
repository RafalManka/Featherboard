use crate::AppState;
use crate::error::AppError;
use crate::org::CurrentOrg;
use async_trait::async_trait;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_sessions::Session;

pub fn github_router() -> Router<AppState> {
    Router::new()
        .route("/", get(auth_github))
        .route("/callback", get(auth_github_callback))
}

#[derive(Deserialize)]
struct GithubAuthQuery {
    code: String,
    state: String,
}

#[derive(Clone, Serialize)]
pub struct GithubAccessToken {
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct GithubToken {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

#[derive(Deserialize)]
pub struct GithubUser {
    pub id: i64,
    pub email: Option<String>,
    pub name: Option<String>,
    pub login: String,
}

#[derive(Deserialize)]
pub struct GithubEmail {
    pub email: String,
    pub primary: bool,
}

async fn auth_github_callback(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Query(query): Query<GithubAuthQuery>,
) -> Result<Response, AppError> {
    let Some(client_secret) = current_org.github_client_secret.clone() else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub login is not configured",
        )
            .into_response());
    };

    let Some(client_id) = current_org.github_client_id.clone() else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub login is not configured",
        )
            .into_response());
    };

    let oauth_state = session.get::<String>("github_oauth_state").await?;
    let Some(oauth_state) = oauth_state else {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    };
    if query.state != oauth_state {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    }

    let response = state
        .github_client
        .access_token(&GithubAccessToken {
            code: query.code,
            redirect_uri: format!(
                "http://localhost:3000/{}/auth/github/callback",
                current_org.slug
            ),
            client_id,
            client_secret,
        })
        .await;

    let Ok(response) = response else {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    };

    let access_token = response.access_token;

    let response = state.github_client.github_user(&access_token).await;

    let Ok(response) = response else {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    };

    let email = match response.email {
        Some(email) => email,
        None => {
            let response = state.github_client.github_email(access_token).await?;
            let Some(response) = response else {
                return Ok((
                    StatusCode::UNAUTHORIZED,
                    "Credentials not valid - email doesnt exist",
                )
                    .into_response());
            };

            response.email
        }
    };

    let sql = r#"
        INSERT INTO users (org_id, provider, provider_user_id, email, name)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (org_id, provider, provider_user_id) DO UPDATE SET
            email = excluded.email,
            name = excluded.name
        RETURNING id
    "#;

    let user_id: i64 = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .bind("github")
        .bind(response.id.to_string())
        .bind(email)
        .bind(response.name.unwrap_or(response.login))
        .fetch_one(&state.db)
        .await?;

    session.insert("user_id", user_id).await?;

    Ok(current_org.redirect("").into_response())
}

async fn auth_github(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    let Some(client_id) = current_org.github_client_id else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub login is not configured",
        )
            .into_response());
    };

    let state = rand::random::<u64>();
    let state = format!("{state:x}");
    session.insert("github_oauth_state", &state).await?;

    let uri = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=read:user+user:email",
        client_id,
        format!(
            "http://localhost:3000/{}/auth/github/callback",
            current_org.slug
        ),
        state
    );

    Ok(Redirect::to(&uri).into_response())
}

#[async_trait]
pub trait GithubClient {
    async fn access_token(&self, form: &GithubAccessToken) -> Result<GithubToken, AppError>;
    async fn github_user(&self, access_token: &str) -> Result<GithubUser, AppError>;
    async fn github_email(&self, access_token: String) -> Result<Option<GithubEmail>, AppError>;
}

#[async_trait]
impl GithubClient for GithubClientImpl {
    async fn access_token(&self, form: &GithubAccessToken) -> Result<GithubToken, AppError> {
        let response = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await?
            .json::<GithubToken>()
            .await?;
        Ok(response)
    }

    async fn github_user(&self, access_token: &str) -> Result<GithubUser, AppError> {
        let response = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "Featherboard")
            .send()
            .await?
            .json::<GithubUser>()
            .await?;
        Ok(response)
    }

    async fn github_email(&self, access_token: String) -> Result<Option<GithubEmail>, AppError> {
        let response = self
            .client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "Featherboard")
            .send()
            .await?
            .json::<Vec<GithubEmail>>()
            .await?
            .into_iter()
            .find(|el| el.primary);
        Ok(response)
    }
}

pub struct GithubClientImpl {
    pub client: Client,
}

impl GithubClientImpl {
    pub fn arc() -> Arc<dyn GithubClient + Send + Sync> {
        Arc::new(GithubClientImpl::new())
    }

    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}
