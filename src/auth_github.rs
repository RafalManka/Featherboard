use crate::AppState;
use crate::error::AppError;
use crate::org::CurrentOrg;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize)]
struct GithubAccessToken {
    code: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GithubToken {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Deserialize)]
struct GithubUser {
    id: i64,
    email: Option<String>,
    name: Option<String>,
    login: String,
}

#[derive(Deserialize)]
struct GithubEmail {
    email: String,
    primary: bool,
}

async fn auth_github_callback(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Query(query): Query<GithubAuthQuery>,
) -> Result<Response, AppError> {
    let Ok(client_secret) = std::env::var("GITHUB_CLIENT_SECRET") else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub login is not configured",
        )
            .into_response());
    };

    let Ok(client_id) = std::env::var("GITHUB_CLIENT_ID") else {
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

    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&GithubAccessToken {
            code: query.code,
            redirect_uri: format!(
                "http://localhost:3000/{}/auth/github/callback",
                current_org.slug
            ),
            client_id,
            client_secret,
        })
        .send()
        .await?
        .json::<GithubToken>()
        .await;

    let Ok(response) = response else {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    };

    let access_token = response.access_token;

    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token.clone()))
        .header("User-Agent", "Featherboard")
        .send()
        .await?
        .json::<GithubUser>()
        .await;

    let Ok(response) = response else {
        return Ok((StatusCode::UNAUTHORIZED, "Credentials not valid").into_response());
    };

    let email = match response.email {
        Some(email) => email,
        None => {
            let response = client
                .get("https://api.github.com/user/emails")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "Featherboard")
                .send()
                .await?
                .json::<Vec<GithubEmail>>()
                .await?
                .into_iter()
                .find(|el| el.primary);

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
        ON CONFLICT (provider, provider_user_id) DO UPDATE SET
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

    Ok(Redirect::to(&current_org.path(String::new())).into_response())
}

async fn auth_github(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    let Ok(client_id) = std::env::var("GITHUB_CLIENT_ID") else {
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
