use featherboard::email::EmailClient;
use featherboard::{app_router, session_store};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::env;
use std::io::IsTerminal;
use std::str::FromStr;
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

    let session_layer = session_store(&db).await;

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
