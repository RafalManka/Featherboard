mod error;
mod static_assets;
mod templates;

use askama::Template;
use axum::extract::State;
use axum::{routing::get, Router};
use error::AppError;
use sqlx::SqlitePool;
use std::io::IsTerminal;
use templates::HtmlTemplate;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_sessions::{Session, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:featherboard.db?mode=rwc".into());
    let db = SqlitePool::connect(&database_url)
        .await
        .expect("failed to connect to database");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");

    let session_store = SqliteStore::new(db.clone());
    session_store
        .migrate()
        .await
        .expect("failed to run session store migrations");
    let session_layer = SessionManagerLayer::new(session_store);

    let app = Router::new()
        .route("/", get(home))
        .route("/healthz", get(healthz))
        .route("/static/{*path}", get(static_assets::serve))
        .with_state(AppState { db })
        .layer(session_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    tracing::info!("listening on {addr}");
    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn healthz(State(state): State<AppState>) -> Result<&'static str, AppError> {
    sqlx::query("SELECT 1").execute(&state.db).await?;
    Ok("ok")
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    visits: i64,
}

async fn home(session: Session) -> Result<HtmlTemplate<HomeTemplate>, AppError> {
    let visits: i64 = session.get("visits").await?.unwrap_or(0) + 1;
    session.insert("visits", visits).await?;
    Ok(HtmlTemplate(HomeTemplate { visits }))
}
