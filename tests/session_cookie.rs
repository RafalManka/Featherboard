mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use tower::ServiceExt;

// Guards against the SameSite=Strict regression from 2026-10-10 (see plan.md,
// Phase 7): that default silently dropped the session cookie on the
// cross-site redirect back from Stripe Checkout, 403ing admins right after a
// real payment succeeded. Voting is just a convenient anonymous, session-
// writing route to trigger a Set-Cookie response with — the subject under
// test is the cookie's SameSite attribute, not voting itself.
#[tokio::test]
async fn session_cookie_has_samesite_lax() {
    let db = test_db().await;

    // Migrations seed org id 1 / slug "default" automatically.
    let sql = r#"
        INSERT INTO ideas (id, org_id, title, description, status)
        VALUES (1, 1, 'Idea 1', 'Idea 1 description', 'open')
    "#;
    sqlx::query(sql).execute(&db).await.unwrap();

    let session_layer = session_store(&db).await;

    let app = app_router(db, session_layer, None, None);

    let request = axum::http::Request::builder()
        .uri("/default/ideas/1/vote")
        .method("POST")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let set_cookie = response
        .headers()
        .get("Set-Cookie")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(set_cookie.contains("SameSite=Lax;"));
}
