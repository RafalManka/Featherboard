use featherboard::app_router;
use featherboard::email::EmailClient;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::env;
use std::io::IsTerminal;
use std::str::FromStr;
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:featherboard.db?mode=rwc".into());

    let options = SqliteConnectOptions::from_str(&database_url)
        .expect("failed to connect to database")
        .pragma("foreign_keys", "ON");

    let db = SqlitePoolOptions::new()
        .connect_with(options)
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

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(365)))
        .with_same_site(SameSite::Lax);

    let email_client = EmailClient::load();
    if email_client.is_none() {
        tracing::warn!("SMTP not configured. Email notifications disabled");
    }
    let public_url = env::var("PUBLIC_URL")
        .ok()
        .map(|e| e.trim_end_matches('/').to_string());

    let app = app_router(db, session_layer, email_client, public_url);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address");

    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await.expect("server error");
}
