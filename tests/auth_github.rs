mod common;

use async_trait::async_trait;
use featherboard::app_router;
use featherboard::auth_github::{
    GithubAccessToken, GithubClient, GithubEmail, GithubToken, GithubUser,
};
use featherboard::error::AppError;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions_sqlx_store::SqliteStore;

use crate::common::test_db;
use tower_sessions::{Session, SessionManagerLayer};

#[tokio::test]
async fn github_accounts_are_org_scoped() {
    let db = test_db().await;

    sqlx::query(
        "UPDATE organizations SET github_client_id = ?, github_client_secret = ? WHERE slug = ?",
    )
    .bind("github-client-key")
    .bind("github-client-secret")
    .bind("default")
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO organizations (id, name, slug, github_client_id, github_client_secret) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(2)
    .bind("Second Org")
    .bind("second")
    .bind("github-client-key")
    .bind("github-client-secret")
    .execute(&db)
    .await
    .unwrap();

    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(store.clone()).with_secure(false);
    let app = app_router(
        db.clone(),
        session_layer,
        None,
        None,
        Arc::new(FakeGithubClient),
    );

    let response = authenticate(&app, &store, "default").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let response = authenticate(&app, &store, "second").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'github' ORDER BY org_id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].0, 1);
    assert_eq!(users[1].0, 2);
    assert_ne!(users[0].1, users[1].1);
}

#[tokio::test]
async fn github_account_is_reused_within_one_org() {
    let db = test_db().await;

    sqlx::query(
        "UPDATE organizations SET github_client_id = ?, github_client_secret = ? WHERE slug = ?",
    )
    .bind("github-client-key")
    .bind("github-client-secret")
    .bind("default")
    .execute(&db)
    .await
    .unwrap();

    let store = SqliteStore::new(db.clone());
    store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(store.clone()).with_secure(false);
    let app = app_router(
        db.clone(),
        session_layer,
        None,
        None,
        Arc::new(FakeGithubClient),
    );

    let response = authenticate(&app, &store, "default").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let response = authenticate(&app, &store, "default").await;
    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);

    let users: Vec<(i64, i64)> =
        sqlx::query_as("SELECT org_id, id FROM users WHERE provider = 'github' ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].0, 1);
}

async fn authenticate(
    app: &axum::Router,
    store: &SqliteStore,
    org_slug: &str,
) -> axum::response::Response {
    let state = format!("{:x}", rand::random::<u64>());
    let session = Session::new(None, Arc::new(store.clone()), None);
    session.insert("github_oauth_state", &state).await.unwrap();
    session.save().await.unwrap();
    let session_id = session.id().unwrap().to_string();

    let request = axum::http::Request::builder()
        .uri(format!(
            "/{org_slug}/auth/github/callback?code=1&state={state}"
        ))
        .header("Cookie", format!("id={session_id}"))
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    app.clone().oneshot(request).await.unwrap()
}

struct FakeGithubClient;

#[async_trait]
impl GithubClient for FakeGithubClient {
    async fn access_token(&self, _form: &GithubAccessToken) -> Result<GithubToken, AppError> {
        Ok(GithubToken {
            access_token: "fake-access-token".to_string(),
            token_type: "bearer".to_string(),
            scope: "read:user user:email".to_string(),
        })
    }

    async fn github_user(&self, _access_token: &str) -> Result<GithubUser, AppError> {
        Ok(GithubUser {
            id: 123,
            email: Some("example@email.com".to_string()),
            name: Some("Example Name".to_string()),
            login: "Example".to_string(),
        })
    }

    async fn github_email(&self, _access_token: String) -> Result<Option<GithubEmail>, AppError> {
        Ok(Some(GithubEmail {
            email: "example@email.com".to_string(),
            primary: true,
        }))
    }
}
