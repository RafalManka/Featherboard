mod common;

use crate::common::test_db;
use featherboard::auth_github::GithubClientImpl;
use featherboard::environment::Environment;
use featherboard::{app_router, session_store};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app(db: &sqlx::SqlitePool) -> axum::Router {
    app_router(
        db.clone(),
        session_store(db).await,
        None,
        Environment::load(),
        GithubClientImpl::arc(),
    )
}

async fn insert_pending_organization(
    db: &sqlx::SqlitePool,
    name: &str,
    slug: &str,
    email: &str,
    expires_at: Option<&str>,
) -> String {
    let token = Uuid::new_v4().to_string();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    let sql = match expires_at {
        Some(_) => {
            "INSERT INTO pending_organizations (name, slug, email, token_hash, expires_at) VALUES (?, ?, ?, ?, ?)"
        }
        None => {
            "INSERT INTO pending_organizations (name, slug, email, token_hash) VALUES (?, ?, ?, ?)"
        }
    };

    let mut query = sqlx::query(sql)
        .bind(name)
        .bind(slug)
        .bind(email)
        .bind(token_hash);

    if let Some(expires_at) = expires_at {
        query = query.bind(expires_at);
    }

    query.execute(db).await.unwrap();
    token
}

#[tokio::test]
async fn valid_setup_link_creates_admin_and_logs_them_in() {
    let db = test_db().await;
    let token = insert_pending_organization(
        &db,
        "Acme",
        "acme",
        "Admin@Example.com",
        None,
    )
    .await;
    let app = setup_app(&db).await;

    let request = axum::http::Request::builder()
        .uri(format!("/setup/confirm/{token}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), "/acme");
    let cookie = response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let settings_request = axum::http::Request::builder()
        .uri("/acme/settings/site")
        .header("Cookie", cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let settings_response = app.oneshot(settings_request).await.unwrap();
    assert_eq!(settings_response.status(), axum::http::StatusCode::OK);

    let organization: (String, String) =
        sqlx::query_as("SELECT name, slug FROM organizations WHERE slug = 'acme'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(organization, ("Acme".to_string(), "acme".to_string()));

    let user: (String, String, String, String) = sqlx::query_as(
        "SELECT email, name, provider, provider_user_id FROM users WHERE org_id = (SELECT id FROM organizations WHERE slug = 'acme')",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        user,
        (
            "admin@example.com".to_string(),
            "admin@example.com".to_string(),
            "email".to_string(),
            "admin@example.com".to_string(),
        )
    );

    let pending_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pending_organizations WHERE slug = 'acme'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(pending_count, 0);
}

#[tokio::test]
async fn expired_setup_link_does_not_create_anything() {
    let db = test_db().await;
    let token = insert_pending_organization(
        &db,
        "Expired",
        "expired",
        "expired@example.com",
        Some("2000-01-01T00:00:00.000Z"),
    )
    .await;
    let app = setup_app(&db).await;

    let request = axum::http::Request::builder()
        .uri(format!("/setup/confirm/{token}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), "/");

    let organization_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE slug = 'expired'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(organization_count, 0);
}

#[tokio::test]
async fn invalid_setup_link_does_not_create_anything() {
    let db = test_db().await;
    let app = setup_app(&db).await;

    let request = axum::http::Request::builder()
        .uri("/setup/confirm/not-a-real-token")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), "/");

    let pending_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pending_organizations")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(pending_count, 0);
}

#[tokio::test]
async fn setup_confirmation_rolls_back_when_organization_creation_fails() {
    let db = test_db().await;
    let token = insert_pending_organization(
        &db,
        "Duplicate",
        "default",
        "duplicate@example.com",
        None,
    )
    .await;
    let app = setup_app(&db).await;

    let request = axum::http::Request::builder()
        .uri(format!("/setup/confirm/{token}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pending_organizations WHERE email = 'duplicate@example.com'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(pending_count, 1);

    let user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE email = 'duplicate@example.com'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(user_count, 0);
}
