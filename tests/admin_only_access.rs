mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::Session;
use tower_sessions_sqlx_store::SqliteStore;

// Guards against a non-admin reaching either the settings page (which now
// shows billing status, replacing the old standalone /subscriptions page
// deleted 2026-10-21) or the subscription checkout action. Verified live
// 2026-10-10 by temporarily flipping a real user's is_admin to 0 and
// confirming both 403'd (see plan.md, Phase 7) — this test covers the
// same two routes without needing to mutate and restore real data.
#[tokio::test]
async fn non_admin_is_forbidden_on_both_subscription_routes() {
    let db = test_db().await;

    // is_admin = 0 — deliberately not an admin.
    sqlx::query(
        "INSERT INTO users (org_id, provider, provider_user_id, email, name, is_admin) \
         VALUES (1, 'github', '1', 'member@example.com', 'Member', 0)",
    )
    .execute(&db)
    .await
    .unwrap();

    let session_layer = session_store(&db).await;

    let store = Arc::new(SqliteStore::new(db.clone()));
    let session = Session::new(None, store, None);
    session.insert("user_id", 1i64).await.unwrap();
    session.save().await.unwrap();
    let session_id = session.id().unwrap().to_string();

    let app = app_router(db, session_layer, None, None);

    let settings_request = axum::http::Request::builder()
        .uri("/default/settings/site")
        .header("Cookie", format!("id={session_id}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let settings_response = app.clone().oneshot(settings_request).await.unwrap();
    assert_eq!(
        settings_response.status(),
        axum::http::StatusCode::FORBIDDEN
    );

    let post_request = axum::http::Request::builder()
        .uri("/default/subscriptions/checkout")
        .method("POST")
        .header("Cookie", format!("id={session_id}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from("idempotency_key=whatever"))
        .unwrap();
    let post_response = app.oneshot(post_request).await.unwrap();
    assert_eq!(post_response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_render_settings_page() {
    let db = test_db().await;

    sqlx::query(
        "INSERT INTO users (org_id, provider, provider_user_id, email, name, is_admin) \
         VALUES (1, 'github', '1', 'admin@example.com', 'Admin', 1)",
    )
    .execute(&db)
    .await
    .unwrap();

    let session_layer = session_store(&db).await;
    let store = Arc::new(SqliteStore::new(db.clone()));
    let session = Session::new(None, store, None);
    session.insert("user_id", 1i64).await.unwrap();
    session.save().await.unwrap();
    let session_id = session.id().unwrap().to_string();

    let app = app_router(db, session_layer, None, None);
    let request = axum::http::Request::builder()
        .uri("/default/settings/site")
        .header("Cookie", format!("id={session_id}"))
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("class=\"settings-layout\""));
    assert!(body.contains("Billing"));
    assert!(body.contains("Billing is not configured for this instance."));
}
