use crate::AppState;
use crate::error::AppError;
use crate::models::SubscriptionStatus;
use crate::subscription::upsert_from_stripe;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn webhooks_router() -> Router<AppState> {
    Router::new().route("/stripe", post(stripe_webhook))
}

#[derive(serde::Deserialize)]
struct StripeEvent {
    #[serde(rename = "type")]
    event_type: String,
}
#[derive(serde::Deserialize)]
struct StripeSubscriptionEvent {
    data: StripeEventData,
}

#[derive(serde::Deserialize)]
struct StripeSessionObject {
    #[serde(default)]
    metadata: HashMap<String, String>,
}

#[derive(serde::Deserialize)]
struct StripeSessionData {
    object: StripeSessionObject,
}

#[derive(serde::Deserialize)]
struct StripeSessionEvent {
    data: StripeSessionData,
}

#[derive(serde::Deserialize)]
struct StripeEventData {
    object: StripeSubscriptionObject,
}

#[derive(serde::Deserialize)]
struct StripeSubscriptionObject {
    id: String,
    customer: String,
    status: String,

    #[serde(default)]
    metadata: HashMap<String, String>,
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
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

    let Ok(event_type) = serde_json::from_slice::<StripeEvent>(&body).map(|e| e.event_type) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid JSON").into_response());
    };

    match event_type.as_str() {
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => {
            handle_subscription_event(&event_type, &body, &state.db).await
        }
        "checkout.session.expired" => handle_session_event(&body, &state.db).await,
        _ => Ok(StatusCode::OK.into_response()),
    }
}

async fn handle_session_event(body: &Bytes, db: &SqlitePool) -> Result<Response, AppError> {
    let Ok(event) = serde_json::from_slice::<StripeSessionEvent>(body) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid subscription event").into_response());
    };

    let Some(idempotency_key) = event.data.object.metadata.get("idempotency_key") else {
        return Ok((StatusCode::BAD_REQUEST, "missing organization metadata").into_response());
    };

    delete_checkout_attempt(db, idempotency_key).await?;

    Ok(StatusCode::OK.into_response())
}

async fn delete_checkout_attempt(
    db: &SqlitePool,
    idempotency_key: &String,
) -> Result<(), AppError> {
    let sql = r#"
        DELETE FROM checkouts_attempts WHERE idempotency_key = ?
    "#;
    sqlx::query(sql).bind(idempotency_key).execute(db).await?;
    Ok(())
}

async fn handle_subscription_event(
    event_type: &str,
    body: &Bytes,
    db: &SqlitePool,
) -> Result<Response, AppError> {
    let Ok(event) = serde_json::from_slice::<StripeSubscriptionEvent>(body) else {
        return Ok((StatusCode::BAD_REQUEST, "invalid subscription event").into_response());
    };

    let Some(status) = SubscriptionStatus::parse(&event.data.object.status) else {
        return Ok((StatusCode::BAD_REQUEST, "unknown subscription status").into_response());
    };

    let Some(org_id) = event.data.object.metadata.get("featherboard_org_id") else {
        return Ok((StatusCode::BAD_REQUEST, "missing organization metadata").into_response());
    };

    let Ok(org_id) = org_id.parse::<i64>() else {
        return Ok((StatusCode::BAD_REQUEST, "invalid organization metadata").into_response());
    };

    let idempotency_key = if matches!(event_type, "customer.subscription.created") {
        let Some(idempotency_key) = event.data.object.metadata.get("idempotency_key") else {
            return Ok((StatusCode::BAD_REQUEST, "missing organization metadata").into_response());
        };
        Some(idempotency_key)
    } else {
        None
    };

    upsert_from_stripe(
        db,
        org_id,
        &event.data.object.customer,
        &event.data.object.id,
        status,
    )
    .await?;

    if let Some(idempotency_key) = idempotency_key {
        delete_checkout_attempt(db, idempotency_key).await?;
    }

    tracing::debug!(
        org_id,
        subscription_id = %event.data.object.id,
        customer_id = %event.data.object.customer,
        status = %status.as_str(),
        prefix = "received event",
        event = %event_type
    );

    Ok(StatusCode::OK.into_response())
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

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_db() -> SqlitePool {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE checkouts_attempts (
                org_id INTEGER NOT NULL,
                idempotency_key TEXT NOT NULL UNIQUE
            )
            "#,
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                org_id INTEGER NOT NULL UNIQUE,
                stripe_customer_id TEXT NOT NULL UNIQUE,
                stripe_subscription_id TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT 'now',
                updated_at TEXT NOT NULL DEFAULT 'now'
            )
            "#,
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    async fn insert_attempt(db: &SqlitePool, org_id: i64, key: &str) {
        sqlx::query("INSERT INTO checkouts_attempts (org_id, idempotency_key) VALUES (?, ?)")
            .bind(org_id)
            .bind(key)
            .execute(db)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn expired_checkout_session_deletes_its_attempt() {
        let db = test_db().await;
        let key = "attempt-key";
        insert_attempt(&db, 1, key).await;
        let body = Bytes::from(format!(
            r#"{{"data":{{"object":{{"metadata":{{"idempotency_key":"{key}"}}}}}}}}"#
        ));

        let response = handle_session_event(&body, &db).await.unwrap();
        let remaining = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM checkouts_attempts WHERE idempotency_key = ?",
        )
        .bind(key)
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, response.status());
        assert_eq!(0, remaining);
    }

    #[tokio::test]
    async fn created_subscription_upserts_and_deletes_its_attempt() {
        let db = test_db().await;
        let key = "attempt-key";
        insert_attempt(&db, 1, key).await;
        let body = Bytes::from(format!(
            r#"{{"data":{{"object":{{"id":"sub_test","customer":"cus_test","status":"active","metadata":{{"featherboard_org_id":"1","idempotency_key":"{key}"}}}}}}}}"#
        ));

        let response = handle_subscription_event("customer.subscription.created", &body, &db)
            .await
            .unwrap();
        let stored_subscription = sqlx::query_scalar::<_, String>(
            "SELECT stripe_subscription_id FROM subscriptions WHERE org_id = 1",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        let remaining_attempts = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM checkouts_attempts WHERE idempotency_key = ?",
        )
        .bind(key)
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(StatusCode::OK, response.status());
        assert_eq!("sub_test", stored_subscription);
        assert_eq!(0, remaining_attempts);
    }

    fn signed_header(body: &[u8], timestamp: i64, secret: &str) -> String {
        let mut signed_payload = timestamp.to_string().into_bytes();
        signed_payload.push(b'.');
        signed_payload.extend_from_slice(body);

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(&signed_payload);

        format!(
            "t={},v1={}",
            timestamp,
            hex::encode(mac.finalize().into_bytes())
        )
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn accepts_valid_signature() {
        let body = br#"{"type":"test"}"#;
        let secret = "whsec_test";
        let header = signed_header(body, current_timestamp(), secret);

        assert!(verify_stripe_signature(body, &header, secret));
    }

    #[test]
    fn rejects_wrong_secret() {
        let body = br#"{"type":"test"}"#;
        let header = signed_header(body, current_timestamp(), "whsec_real");

        assert!(!verify_stripe_signature(body, &header, "whsec_wrong"));
    }

    #[test]
    fn rejects_malformed_header() {
        let body = br#"{"type":"test"}"#;

        assert!(!verify_stripe_signature(
            body,
            "t=not-a-number",
            "whsec_test"
        ));
        assert!(!verify_stripe_signature(body, "t=123,v1", "whsec_test"));
    }

    #[test]
    fn rejects_expired_timestamp() {
        let body = br#"{"type":"test"}"#;
        let secret = "whsec_test";
        let old_timestamp = current_timestamp() - TOLERANCE_SECONDS - 1;
        let header = signed_header(body, old_timestamp, secret);

        assert!(!verify_stripe_signature(body, &header, secret));
    }
}
