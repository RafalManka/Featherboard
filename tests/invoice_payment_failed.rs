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

// Guards against two real bugs found and fixed 2026-10-11/12 (see plan.md,
// Phase 7): (1) treating a Stripe subscription id like "sub_xxx" as a number
// via `.parse::<i64>()`, which would 400 on every real event; (2) reading
// `org_id` (an INTEGER column) into a Rust String via `query_scalar`, which
// compiles clean but fails at runtime — `cargo check` can't see either of
// these, since both only surface when real data actually flows through the
// runtime-checked sqlx query API. A real Stripe subscription id (not a
// plain number) is used deliberately below to make sure bug #1 can't creep
// back in unnoticed.
#[tokio::test]
async fn invoice_payment_failed_resolves_org_and_returns_ok() {
    let secret = "whsec_test_secret";
    // SAFETY: this test is the only test in this binary.
    unsafe {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", secret);
    }

    let db = test_db().await;

    sqlx::query(
        "INSERT INTO subscriptions (org_id, stripe_customer_id, stripe_subscription_id, status) \
         VALUES (1, 'cus_test', 'sub_1UOxcIPSAteXyg1hdaZg1Sqg', 'active')",
    )
    .execute(&db)
    .await
    .unwrap();

    let session_layer = session_store(&db).await;
    let app = app_router(db, session_layer, None, None);

    let body = br#"{"type":"invoice.payment_failed","data":{"object":{"subscription":"sub_1UOxcIPSAteXyg1hdaZg1Sqg"}}}"#.to_vec();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
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

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
