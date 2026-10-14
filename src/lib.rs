use crate::auth::auth_router;
use crate::changelog::changelog_router;
use crate::email::EmailClient;
use crate::error::AppError;
use crate::ideas::{ideas_router, list_ideas, roadmap};
use crate::login::login_router;
use crate::subscription::subscriptions_router;
use crate::webhooks::webhooks_router;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use sqlx::{Pool, Sqlite, SqlitePool};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;

pub mod admin;
pub mod auth;
pub mod auth_github;
pub mod changelog;
pub mod comments;
pub mod email;
pub mod error;
pub mod ideas;
pub mod login;
pub mod models;
pub mod org;
pub mod static_assets;
pub mod subscription;
pub mod templates;
pub mod validation;
pub mod webhooks;

#[derive(Clone)]
pub struct AppState {
    pub public_url: Option<String>,
    pub db: SqlitePool,
    pub email_client: Option<EmailClient>,
}

pub fn app_router(
    db: Pool<Sqlite>,
    session_layer: SessionManagerLayer<SqliteStore>,
    email_client: Option<EmailClient>,
    public_url: Option<String>,
) -> Router {
    let app = Router::new()
        .route("/", get(root_redirect))
        .nest("/{slug}", org_router())
        .nest("/webhooks", webhooks_router())
        .route("/static/{*path}", get(static_assets::serve))
        .route("/healthz", get(healthz))
        .with_state(AppState {
            public_url,
            db,
            email_client,
        })
        .layer(session_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );
    app
}

fn org_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_ideas))
        .route("/roadmap", get(roadmap))
        .nest("/ideas", ideas_router())
        .nest("/changelogs", changelog_router())
        .nest("/auth", auth_router())
        .nest("/subscriptions", subscriptions_router())
        .merge(login_router())
}

pub async fn root_redirect(State(state): State<AppState>) -> Result<Response, AppError> {
    let sql = r#"
        SELECT slug
        FROM organizations
        ORDER BY id ASC
        LIMIT 1
    "#;

    let slug = sqlx::query_scalar::<_, String>(&sql)
        .fetch_optional(&state.db)
        .await?;

    match slug {
        None => Ok((StatusCode::NOT_FOUND, "organization not found").into_response()),
        Some(slug) => Ok(Redirect::to(&format!("/{}", slug)).into_response()),
    }
}

pub async fn healthz(State(state): State<AppState>) -> Result<&'static str, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok("ok")
}
