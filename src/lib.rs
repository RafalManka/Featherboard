use crate::auth::auth_router;
use crate::auth_github::GithubClient;
use crate::changelog::changelog_router;
use crate::email::Mailer;
use crate::environment::Environment;
use crate::error::AppError;
use crate::ideas::{ideas_router, list_ideas, roadmap};
use crate::login::login_router;
use crate::settings::settings_router;
use crate::setup::setup_router;
use crate::subscription::subscriptions_router;
use crate::webhooks::webhooks_router;
use axum::Router;
#[cfg(feature = "e2e")]
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
#[cfg(feature = "e2e")]
use axum::routing::post;
use sqlx::{Pool, Sqlite, SqlitePool};
use std::sync::Arc;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;

pub mod admin;
pub mod auth;
pub mod auth_github;
mod billing;
pub mod changelog;
pub mod comments;
pub mod email;
pub mod environment;
pub mod error;
pub mod ideas;
pub mod login;
pub mod models;
pub mod org;
mod plan;
pub mod server;
mod settings;
pub mod setup;
pub mod site_settings;
pub mod static_assets;
pub mod subscription;
pub mod templates;
pub mod user_settings;
pub mod validation;
pub mod webhooks;

#[derive(Clone)]
pub struct AppState {
    pub environment: Environment,
    pub db: SqlitePool,
    pub email_client: Option<Arc<dyn Mailer + Send + Sync>>,
    pub github_client: Arc<dyn GithubClient + Send + Sync>,
}

pub async fn session_store(db: &Pool<Sqlite>) -> SessionManagerLayer<SqliteStore> {
    let session_store = SqliteStore::new(db.clone());
    session_store
        .migrate()
        .await
        .expect("failed to run session store migrations");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(365)))
        .with_same_site(SameSite::Lax);
    session_layer
}

pub fn app_router(
    db: Pool<Sqlite>,
    session_layer: SessionManagerLayer<SqliteStore>,
    email_client: Option<Arc<dyn Mailer + Send + Sync>>,
    environment: Environment,
    github_client: Arc<dyn GithubClient + Send + Sync>,
) -> Router {
    let app = Router::new().route("/", get(root_redirect));

    #[cfg(feature = "e2e")]
    let app = app.route("/e2e/billing/{slug}/{status}", post(e2e_set_billing_status));

    let app = app
        .nest("/{slug}", org_router())
        .nest("/webhooks", webhooks_router())
        .route("/static/{*path}", get(static_assets::serve))
        .route("/healthz", get(healthz))
        .merge(if email_client.is_some() {
            Router::new().nest("/setup", setup_router())
        } else {
            Router::new()
        })
        .with_state(AppState {
            environment,
            db,
            email_client,
            github_client,
        })
        .layer(session_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );
    app
}

#[cfg(feature = "e2e")]
async fn e2e_set_billing_status(
    State(state): State<AppState>,
    Path((slug, status)): Path<(String, String)>,
) -> Result<StatusCode, error::AppError> {
    let org_id: Option<i64> = sqlx::query_scalar("SELECT id FROM organizations WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?;

    let Some(org_id) = org_id else {
        return Ok(StatusCode::NOT_FOUND);
    };

    if status == "free" {
        sqlx::query("DELETE FROM subscriptions WHERE org_id = ?")
            .bind(org_id)
            .execute(&state.db)
            .await?;
        return Ok(StatusCode::OK);
    }

    let Some(status) = models::SubscriptionStatus::parse(&status) else {
        return Ok(StatusCode::BAD_REQUEST);
    };

    subscription::upsert_from_stripe(
        &state.db,
        org_id,
        &format!("e2e-customer-{org_id}"),
        &format!("e2e-subscription-{org_id}"),
        status,
        1,
    )
    .await?;

    Ok(StatusCode::OK)
}

fn org_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_ideas))
        .route("/roadmap", get(roadmap))
        .nest("/ideas", ideas_router())
        .nest("/changelogs", changelog_router())
        .nest("/auth", auth_router())
        .nest("/subscriptions", subscriptions_router())
        .nest("/settings", settings_router())
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
