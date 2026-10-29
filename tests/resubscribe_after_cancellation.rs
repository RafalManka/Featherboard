mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::Session;
use tower_sessions_sqlx_store::SqliteStore;

// Guards against the resubscribe-after-cancellation bug fixed 2026-10-10
// (see plan.md, Phase 7): once a subscription's status became `canceled`,
// the UI got stuck permanently showing the word "canceled" with no way
// back to Subscribe, even though the backend (has_ongoing_subscription)
// already allowed a fresh checkout for that exact status. This confirms
// the Subscribe form actually renders for a canceled org, not just that
// the page loads. Targets /settings (not the old standalone /subscriptions
// page, deleted 2026-10-21 once Settings' Billing section replaced it) —
// both rendered the same SubscriptionView through the same shared
// get_subscription_view helper, so the guarantee is unchanged.
#[tokio::test]
async fn canceled_subscription_shows_subscribe_form() {
    // SAFETY: this test is the only test in this binary (each tests/*.rs
    // file is its own process), so there's no concurrent access to worry
    // about.
    unsafe {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("STRIPE_PRICE_ID", "price_dummy");
    }

    let db = test_db().await;

    // Migrations seed org id 1 / slug "default" automatically.
    sqlx::query(
        "INSERT INTO users (org_id, provider, provider_user_id, email, name, is_admin) \
         VALUES (1, 'github', '1', 'admin@example.com', 'Admin', 1)",
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO subscriptions (org_id, stripe_customer_id, stripe_subscription_id, status) \
         VALUES (1, 'cus_test', 'sub_test', 'canceled')",
    )
    .execute(&db)
    .await
    .unwrap();

    // session_store() runs the session store's own migration (creating the
    // tower_sessions table) as a side effect, so it has to run before we can
    // build a Session against the same store below.
    let session_layer = session_store(&db).await;

    // Build a logged-in admin session directly against the same store the
    // app uses, bypassing real GitHub OAuth (nothing under test here cares
    // about the login flow itself, only what the subscriptions page renders
    // once logged in).
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

    assert!(
        body.contains("Subscribe"),
        "expected a Subscribe form for a canceled subscription, got: {body}"
    );
}
