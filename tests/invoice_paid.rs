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

// Companion to invoice_payment_failed.rs: `handle_invoice_paid` reuses the
// same org-resolution plumbing (find_org_id_by_subscription_id,
// CurrentOrg::from_id), built and verified live 2026-10-12 immediately
// after the payment-failed path (see plan.md, Phase 7). A realistic
// non-numeric subscription id is used for the same reason as the
// payment-failed test: guards against ever re-parsing a Stripe id as a
// number.
#[tokio::test]
async fn invoice_paid_resolves_org_and_returns_ok() {
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

    let body = br#"{"type":"invoice.paid","data":{"object":{"subscription":"sub_1UOxcIPSAteXyg1hdaZg1Sqg"}}}"#.to_vec();
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
