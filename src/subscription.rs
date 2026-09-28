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
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use sqlx::SqlitePool;
use tower_sessions::Session;

pub fn subscriptions_router() -> Router<AppState> {
    Router::new().route("/", get(get_current_subscription))
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
