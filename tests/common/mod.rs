use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

pub async fn test_db() -> Pool<Sqlite> {
    let db = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");
    db
}

pub async fn session_layer(db: &Pool<Sqlite>) -> SessionManagerLayer<SqliteStore> {
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
