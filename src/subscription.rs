use crate::AppState;
use crate::admin::is_admin;
use crate::error::AppError;
use crate::models::SubscriptionStatus;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::env;
use tower_sessions::Session;

pub fn subscriptions_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_current_subscription))
        .route("/checkout", post(stripe_checkout))
}

#[derive(Deserialize)]
struct CheckoutSessionResponse {
    url: Option<String>,
}

async fn stripe_checkout(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let secret_key = match env::var("STRIPE_SECRET_KEY") {
        Ok(secret) => secret,
        Err(_) => return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    };
    let price_id = match env::var("STRIPE_PRICE_ID") {
        Ok(secret) => secret,
        Err(_) => return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    };

    let Some(public_url) = state.public_url else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    let success_url = format!(
        "{}{}",
        public_url,
        current_org.path("/subscriptions".to_string())
    );
    let cancel_url = format!(
        "{}{}",
        public_url,
        current_org.path("/subscriptions".to_string())
    );

    let form = [
        ("success_url", success_url),
        ("cancel_url", cancel_url),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1".to_owned()),
        ("mode", "subscription".to_owned()),
        (
            "subscription_data[metadata][featherboard_org_id]",
            current_org.id.to_string(),
        ),
    ];

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(secret_key, Some(""))
        .form(&form)
        .send()
        .await?
        .error_for_status()?
        .json::<CheckoutSessionResponse>()
        .await?;

    let checkout_url = response.url;

    let Some(checkout_url) = checkout_url else {
        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    Ok(Redirect::to(checkout_url.as_str()).into_response())
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Subscription {
    pub id: i64,
    pub org_id: i64,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Subscription {
    pub fn created_date(&self) -> &str {
        self.created_at.get(..10).unwrap_or(&self.created_at)
    }
}

#[derive(Template)]
#[template(path = "pages/subscription.html")]
struct SubscriptionTemplate {
    layout: Layout,
    subscription: Option<Subscription>,
}

async fn get_current_subscription(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_admin(&session, &state.db, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let subscription = find_by_org_id(&state.db, current_org.id).await?;

    Ok(HtmlTemplate(SubscriptionTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        subscription,
    })
    .into_response())
}

async fn find_by_org_id(db: &SqlitePool, org_id: i64) -> Result<Option<Subscription>, AppError> {
    let sql = r#"
        SELECT id, org_id, stripe_customer_id, stripe_subscription_id, status, created_at, updated_at
        FROM subscriptions
        WHERE org_id = ?
    "#;

    let subscription = sqlx::query_as::<_, Subscription>(sql)
        .bind(org_id)
        .fetch_optional(db)
        .await?;

    Ok(subscription)
}

pub async fn upsert_from_stripe(
    db: &SqlitePool,
    org_id: i64,
    customer_id: &str,
    subscription_id: &str,
    status: SubscriptionStatus,
) -> Result<Subscription, AppError> {
    let sql = r#"
        INSERT INTO subscriptions (
            org_id,
            stripe_customer_id,
            stripe_subscription_id,
            status
        )
        VALUES (?, ?, ?, ?)
        ON CONFLICT (org_id) DO UPDATE SET
            stripe_customer_id = excluded.stripe_customer_id,
            stripe_subscription_id = excluded.stripe_subscription_id,
            status = excluded.status,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        RETURNING
            id,
            org_id,
            stripe_customer_id,
            stripe_subscription_id,
            status,
            created_at,
            updated_at
    "#;

    let result = sqlx::query_as::<_, Subscription>(sql)
        .bind(org_id)
        .bind(customer_id)
        .bind(subscription_id)
        .bind(status.as_str())
        .fetch_one(db)
        .await?;

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    struct TestData {
        org_id: i64,
        customer_id: &'static str,
        subscription_id: &'static str,
        status: SubscriptionStatus,
    }

    static TEST_DATA: &[TestData] = &[
        TestData {
            org_id: 1,
            customer_id: "2",
            subscription_id: "3",
            status: SubscriptionStatus::Active,
        },
        TestData {
            org_id: 2,
            customer_id: "5",
            subscription_id: "6",
            status: SubscriptionStatus::Canceled,
        },
        TestData {
            org_id: 1,
            customer_id: "8",
            subscription_id: "9",
            status: SubscriptionStatus::Unpaid,
        },
        TestData {
            org_id: 1,
            customer_id: "2",
            subscription_id: "9",
            status: SubscriptionStatus::Canceled,
        },
    ];

    #[tokio::test]
    async fn upserts_from_stripe() {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
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
        .expect("Failed to create table");

        for test_case in TEST_DATA {
            let subscription = upsert_from_stripe(
                &db,
                test_case.org_id,
                test_case.customer_id,
                test_case.subscription_id,
                test_case.status,
            )
            .await
            .expect("Error upserting customer");

            assert_eq!(subscription.org_id, test_case.org_id);
            assert_eq!(subscription.stripe_customer_id, test_case.customer_id);
            assert_eq!(
                subscription.stripe_subscription_id,
                test_case.subscription_id
            );
            assert_eq!(subscription.status, test_case.status.as_str());
        }
    }
}
