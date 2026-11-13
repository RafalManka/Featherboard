mod common;

use featherboard::app_router;
use featherboard::auth_github::GithubClientImpl;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use tower_sessions_sqlx_store::SqliteStore;
use uuid::Uuid;

use crate::common::test_db;
use tower_sessions::SessionManagerLayer;

#[tokio::test]
async fn email_accounts_are_org_scoped() {
    let db = test_db().await;

    sqlx::query("INSERT INTO organizations (id, name, slug) VALUES (?, ?, ?)")
        .bind(2)
        .bind("Second Org")
        .bind("second")
        .execute(&db)
        .await
        .unwrap();

    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let app = app_router(
        db.clone(),
        SessionManagerLayer::new(store).with_secure(false),
        None,
        None,
        GithubClientImpl::arc(),
    );

    let response = confirm_email(&app, &db, 1, "default", "user@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let response = confirm_email(&app, &db, 2, "second", "user@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'email' ORDER BY org_id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].0, 1);
    assert_eq!(users[1].0, 2);
    assert_ne!(users[0].1, users[1].1);
}

#[tokio::test]
async fn email_account_is_reused_within_one_org() {
    let db = test_db().await;
    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let app = app_router(
        db.clone(),
        SessionManagerLayer::new(store).with_secure(false),
        None,
        None,
        GithubClientImpl::arc(),
    );

    let response = confirm_email(&app, &db, 1, "default", "user@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let response = confirm_email(&app, &db, 1, "default", "user@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'email' ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].0, 1);
}

#[tokio::test]
async fn email_login_normalizes_email_identity() {
    let db = test_db().await;
    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let app = app_router(
        db.clone(),
        SessionManagerLayer::new(store).with_secure(false),
        None,
        None,
        GithubClientImpl::arc(),
    );

    let response = confirm_email(&app, &db, 1, "default", "User@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let response = confirm_email(&app, &db, 1, "default", "user@example.com").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let users: Vec<(i64, String)> =
        sqlx::query_as("SELECT org_id, email FROM users WHERE provider = 'email' ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users, vec![(1, "user@example.com".to_string())]);
}

async fn confirm_email(
    app: &axum::Router,
    db: &sqlx::SqlitePool,
    org_id: i64,
    org_slug: &str,
    email: &str,
) -> axum::response::Response {
    let token = Uuid::new_v4().to_string();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    sqlx::query("INSERT INTO magic_links (org_id, email, token_hash) VALUES (?, ?, ?)")
        .bind(org_id)
        .bind(email)
        .bind(token_hash)
        .execute(db)
        .await
        .unwrap();

    let request = axum::http::Request::builder()
        .uri(format!("/{org_slug}/login/check-email/{token}"))
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn reject_expired_magic_link() {
    let db = test_db().await;

    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let app = app_router(
        db.clone(),
        SessionManagerLayer::new(store).with_secure(false),
        None,
        None,
        GithubClientImpl::arc(),
    );

    let token = Uuid::new_v4().to_string();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    sqlx::query("INSERT INTO magic_links (org_id, email, token_hash, expires_at) VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-1 second'))")
        .bind(1)
        .bind("user@example.com")
        .bind(token_hash)
        .execute(&db)
        .await
        .unwrap();

    let request = axum::http::Request::builder()
        .uri(format!("/default/login/check-email/{}", token))
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'email' ORDER BY org_id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 0);
}

#[tokio::test]
async fn reject_reused_magic_link() {
    let db = test_db().await;
    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let app = app_router(
        db.clone(),
        SessionManagerLayer::new(store).with_secure(false),
        None,
        None,
        GithubClientImpl::arc(),
    );

    let token = Uuid::new_v4().to_string();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    sqlx::query("INSERT INTO magic_links (org_id, email, token_hash) VALUES (?, ?, ?)")
        .bind(1)
        .bind("email@example.com")
        .bind(token_hash)
        .execute(&db)
        .await
        .unwrap();

    let request = axum::http::Request::builder()
        .uri(format!("/default/login/check-email/{token}"))
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let request = axum::http::Request::builder()
        .uri(format!("/default/login/check-email/{token}"))
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'email' ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].0, 1);
}
