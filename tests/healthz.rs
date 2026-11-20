mod common;

use crate::common::{test_db, test_environment};
use featherboard::auth_github::GithubClientImpl;
use featherboard::{app_router, session_store};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok() {
    let db = test_db().await;
    let session_layer = session_store(&db).await;

    let app = app_router(
        db,
        session_layer,
        None,
        test_environment(),
        GithubClientImpl::arc(),
    );

    let request = axum::http::Request::builder()
        .uri("/healthz")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn fresh_database_hides_setup_without_email() {
    let db = test_db().await;
    let session_layer = session_store(&db).await;

    let app = app_router(
        db,
        session_layer,
        None,
        test_environment(),
        GithubClientImpl::arc(),
    );

    let healthz_request = axum::http::Request::builder()
        .uri("/healthz")
        .body(axum::body::Body::empty())
        .unwrap();
    let healthz_response = app.clone().oneshot(healthz_request).await.unwrap();
    assert_eq!(healthz_response.status(), axum::http::StatusCode::OK);

    let setup_request = axum::http::Request::builder()
        .uri("/setup")
        .body(axum::body::Body::empty())
        .unwrap();
    let setup_response = app.oneshot(setup_request).await.unwrap();
    assert_eq!(setup_response.status(), axum::http::StatusCode::NOT_FOUND);
}
