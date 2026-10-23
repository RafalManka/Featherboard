mod common;

use crate::common::test_db;
use featherboard::{app_router, session_store};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tower::ServiceExt;

//noinspection RsUnresolvedPath
fn sign(body: &[u8], secret: &str, timestamp: i64) -> String {
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(body);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&signed_payload);

    format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    )
}

// Scenario 15 (see plan.md, Phase 7 and the manual test plan): disputes and
// early fraud warnings are deliberately NOT given custom handling here —
// Stripe already emails the account owner by default for both
// charge.dispute.created and radar.early_fraud_warning.created, reaching
// exactly the right audience (the platform operator, not an org admin)
// with more context (Dashboard links, response deadlines) than a
// Featherboard-side email could easily replicate. charge.refunded isn't an
// alert-style event at all. This test proves the fallback catch-all in
// stripe_webhook's match actually behaves correctly for all three real
// event type strings — a signature-verified, well-formed event for a type
// this app doesn't specifically handle should return 200, not error.
#[tokio::test]
async fn risk_events_are_accepted_without_custom_handling() {
    let secret = "whsec_test_secret";
    // SAFETY: this test is the only test in this binary.
    unsafe {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", secret);
    }

    let db = test_db().await;
    let session_layer = session_store(&db).await;
    let app = app_router(db, session_layer, None, None);

    for event_type in [
        "charge.dispute.created",
        "charge.refunded",
        "radar.early_fraud_warning.created",
    ] {
        let body = format!(r#"{{"type":"{event_type}","data":{{"object":{{}}}}}}"#).into_bytes();
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

        let response = app.clone().oneshot(request).await.unwrap();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "{event_type} should be accepted (200) via the fallback catch-all"
        );
    }
}
