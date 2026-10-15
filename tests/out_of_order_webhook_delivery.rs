mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tower::ServiceExt;

fn sign(body: &[u8], secret: &str, timestamp: i64) -> String {
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(body);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&signed_payload);

    format!("t={timestamp},v1={}", hex::encode(mac.finalize().into_bytes()))
}

// Stripe doesn't guarantee delivery order — a delayed event can arrive after
// a newer one (e.g. a retried past_due event landing after a later active
// event already updated local state). upsert_from_stripe's ON CONFLICT ...
// WHERE excluded.stripe_event_created_at >= stripe_event_created_at guard
// (see plan.md, Phase 7) rejects any event whose Stripe-side `created`
// timestamp is older than what's already applied, using Stripe's own event
// timestamp as the ordering signal rather than trusting request-arrival
// order. A stale/rejected event is a successful no-op (200), not an error —
// retrying it would never succeed, so treating it as a failure would just
// make Stripe retry forever for no reason.
#[tokio::test]
async fn stale_event_is_ignored_in_favor_of_newer_already_applied_state() {
    let secret = "whsec_test_secret";
    // SAFETY: this test is the only test in this binary.
    unsafe {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", secret);
    }

    let db = test_db().await;
    let session_layer = session_store(&db).await;
    let app = app_router(db.clone(), session_layer, None, None);

    let send = |app: axum::Router, created: i64, status: &str| {
        let body = format!(
            r#"{{"type":"customer.subscription.updated","created":{created},"data":{{"object":{{"id":"sub_test","customer":"cus_test","status":"{status}","metadata":{{"featherboard_org_id":"1"}}}}}}}}"#
        )
        .into_bytes();
        // Signature timestamp just needs to be within the 5-minute
        // tolerance of now — it's unrelated to the event's own `created`
        // field, which is what the ordering guard actually checks.
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let signature = sign(&body, secret, timestamp);
        let request = axum::http::Request::builder()
            .uri("/webhooks/stripe")
            .method("POST")
            .header("Stripe-Signature", signature)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(body))
            .unwrap();
        app.oneshot(request)
    };

    // Newer event first: active, created = 2000.
    let response = send(app.clone(), 2000, "active").await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    // Stale event arrives after: past_due, created = 1000 — older than
    // what's already applied.
    let response = send(app.clone(), 1000, "past_due").await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let status: String = sqlx::query_scalar("SELECT status FROM subscriptions WHERE org_id = 1")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(
        status, "active",
        "a stale out-of-order event must not override newer already-applied state"
    );
}
