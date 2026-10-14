mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok() {
    let db = test_db().await;
    let session_layer = session_store(&db).await;

    let app = app_router(db, session_layer, None, None);

    let request = axum::http::Request::builder()
        .uri("/healthz")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}
