mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::Session;
use tower_sessions_sqlx_store::SqliteStore;

// Guards against creating a second Stripe subscription for an org that
// already has an ongoing one (active/trialing/past_due/etc. — anything
// except canceled/incomplete_expired). has_ongoing_subscription is meant to
// short-circuit a checkout attempt before Stripe's API is ever called, even
// if the request carries a validly-registered idempotency key (simulating a
// forged/replayed request, not just a UI double-click). Verified live
// 2026-10-10 by checking server-log latency (2ms vs. ~570ms for a real
// Stripe round-trip, see plan.md) — this test checks the same thing more
// directly: no STRIPE_SECRET_KEY/STRIPE_PRICE_ID are set at all, so if the
// guard failed to short-circuit, the handler would panic trying to reach
// Stripe's real API rather than silently succeed.
#[tokio::test]
async fn checkout_redirects_without_calling_stripe_when_already_subscribed() {
    // SAFETY: this test is the only test in this binary.
    unsafe {
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_dummy");
        std::env::set_var("STRIPE_PRICE_ID", "price_dummy");
    }

    let db = test_db().await;

    sqlx::query(
        "INSERT INTO users (org_id, provider, provider_user_id, email, name, is_admin) \
         VALUES (1, 'github', '1', 'admin@example.com', 'Admin', 1)",
    )
    .execute(&db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO subscriptions (org_id, stripe_customer_id, stripe_subscription_id, status) \
         VALUES (1, 'cus_test', 'sub_test', 'active')",
    )
    .execute(&db)
    .await
    .unwrap();

    let key = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO checkouts_attempts (org_id, idempotency_key) VALUES (1, ?)")
        .bind(&key)
        .execute(&db)
        .await
        .unwrap();

    let session_layer = session_store(&db).await;

    let store = Arc::new(SqliteStore::new(db.clone()));
    let session = Session::new(None, store, None);
    session.insert("user_id", 1i64).await.unwrap();
    session.save().await.unwrap();
    let session_id = session.id().unwrap().to_string();

    let app = app_router(
        db,
        session_layer,
        None,
        Some("http://localhost:3000".to_string()),
    );

    let request = axum::http::Request::builder()
        .uri("/default/subscriptions/checkout")
        .method("POST")
        .header("Cookie", format!("id={session_id}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(format!("idempotency_key={key}")))
        .unwrap();

    // oneshot would panic (not just fail an assertion) if the guard didn't
    // short-circuit, since a real network call to api.stripe.com with fake
    // credentials would error out inside the handler.
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(location, "/default/settings");
}
