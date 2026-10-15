mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn sign(body: &[u8], secret: &str, timestamp: i64) -> String {
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(body);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&signed_payload);

    format!("t={timestamp},v1={}", hex::encode(mac.finalize().into_bytes()))
}

// Stripe explicitly does not guarantee exactly-once webhook delivery —
// retries happen on timeout/5xx, so the same event can arrive twice. This
// confirms upsert_from_stripe's `ON CONFLICT (org_id) DO UPDATE SET ... =
// excluded.*` shape is genuinely idempotent against a real replay (not just
// "idempotent by inspection" — see plan.md, Phase 7's webhook-correctness
// item), by sending the identical signed payload twice and checking the
// subscriptions table ends up with exactly one row, not two, and no error
// on the second delivery.
#[tokio::test]
async fn replayed_subscription_created_event_does_not_duplicate_or_error() {
    let secret = "whsec_test_secret";
    // SAFETY: this test is the only test in this binary.
    unsafe {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", secret);
    }

    let db = test_db().await;
    let session_layer = session_store(&db).await;
    let app = app_router(db.clone(), session_layer, None, None);

    let body = br#"{"type":"customer.subscription.created","data":{"object":{"id":"sub_test","customer":"cus_test","status":"active","metadata":{"featherboard_org_id":"1","idempotency_key":"attempt-key"}}}}"#.to_vec();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let signature = sign(&body, secret, timestamp);

    let build_request = || {
        axum::http::Request::builder()
            .uri("/webhooks/stripe")
            .method("POST")
            .header("Stripe-Signature", signature.clone())
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(body.clone()))
            .unwrap()
    };

    let first_response = app.clone().oneshot(build_request()).await.unwrap();
    assert_eq!(first_response.status(), axum::http::StatusCode::OK);

    let second_response = app.oneshot(build_request()).await.unwrap();
    assert_eq!(second_response.status(), axum::http::StatusCode::OK);

    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subscriptions WHERE org_id = 1")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(row_count, 1);

    let status: String =
        sqlx::query_scalar("SELECT status FROM subscriptions WHERE org_id = 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(status, "active");
}
