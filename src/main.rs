mod admin;
mod comments;
mod error;
mod ideas;
mod models;
mod static_assets;
mod templates;
mod validation;

use crate::admin::admin_router;
use crate::ideas::{ideas_router, list_ideas};
use axum::extract::State;
use axum::{Router, routing::get};
use error::AppError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::io::IsTerminal;
use std::str::FromStr;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    default_org_id: i64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:featherboard.db?mode=rwc".into());

    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("failed to connect to database")
        .pragma("foreign_keys", "ON");

    let db = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let default_org_id: i64 = sqlx::query_scalar("SELECT id FROM organizations WHERE slug = 'default'")
        .fetch_one(&db)
        .await
        .expect("default organization not seeded");

    let session_store = SqliteStore::new(db.clone());
    session_store
        .migrate()
        .await
        .expect("failed to run session store migrations");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(365)));

    let app = Router::new()
        .route("/", get(list_ideas))
        .route("/healthz", get(healthz))
        .route("/static/{*path}", get(static_assets::serve))
        .nest("/ideas", ideas_router())
        .merge(admin_router())
        .with_state(AppState { db, default_org_id })
        .layer(session_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address");

    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await.expect("server error");
}

async fn healthz(State(state): State<AppState>) -> Result<&'static str, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok("ok")
}
