use crate::AppState;
use crate::error::AppError;
use axum::Router;
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn webhooks_router() -> Router<AppState> {
    Router::new().route("/stripe", post(stripe_webhook))
}

async fn stripe_webhook(headers: HeaderMap, body: Bytes) -> Result<Response, AppError> {
    let Some(signature) = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
    else {
        return Ok((StatusCode::BAD_REQUEST, "missing Stripe-Signature header").into_response());
    };

    let secret = match env::var("STRIPE_WEBHOOK_SECRET") {
        Ok(secret) => secret,
        Err(_) => return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    };

    if !verify_stripe_signature(&body, signature, &secret) {
        return Ok((StatusCode::BAD_REQUEST, "invalid signature").into_response());
    }

    Ok(StatusCode::NOT_IMPLEMENTED.into_response())
}

const TOLERANCE_SECONDS: i64 = 300;

fn verify_stripe_signature(body: &[u8], header: &str, secret: &str) -> bool {
    let mut t = None;
    let mut signatures = Vec::new();

    for part in header.split(",") {
        let Some((key, value)) = part.split_once("=") else {
            return false;
        };

        match key {
            "t" => t = Some(value),
            "v1" => signatures.push(value),
            _ => (),
        }
    }

    let Some(timestamp) = t else { return false };
    let Ok(timestamp_seconds) = timestamp.parse::<i64>() else {
        return false;
    };
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return false;
    };
    let now = now.as_secs() as i64;
    if now.abs_diff(timestamp_seconds) > TOLERANCE_SECONDS as u64 {
        return false;
    }

    if signatures.is_empty() {
        return false;
    };

    let mut signed_payload = Vec::new();
    signed_payload.extend_from_slice(timestamp.as_bytes());
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(body);

    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };

    mac.update(&signed_payload);

    signatures.iter().any(|signature| {
        let Ok(signature_bytes) = hex::decode(signature) else {
            return false;
        };

        mac.clone().verify_slice(&signature_bytes).is_ok()
    })
}
