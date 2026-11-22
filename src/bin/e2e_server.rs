use featherboard::email::Mailer;
use featherboard::environment::{Environment, StripeConfig};
use featherboard::error::AppError;
use featherboard::server::build_app;
use lettre::Message;
use std::sync::Arc;

const EMAIL_FILE: &str = "/private/tmp/featherboard-e2e-email.eml";
const DATABASE_FILE: &str = "/private/tmp/featherboard-e2e.db";

#[tokio::main]
async fn main() {
    let email_file = std::env::var("E2E_EMAIL_FILE").unwrap_or_else(|_| EMAIL_FILE.to_string());
    let _ = std::fs::remove_file(&email_file);

    let database_file =
        std::env::var("E2E_DATABASE_FILE").unwrap_or_else(|_| DATABASE_FILE.to_string());
    let port = std::env::var("E2E_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3101);
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));

    let _ = std::fs::remove_file(&database_file);
    let _ = std::fs::remove_file(format!("{database_file}-shm"));
    let _ = std::fs::remove_file(format!("{database_file}-wal"));

    let stripe = match (
        std::env::var("STRIPE_SECRET_KEY").ok(),
        std::env::var("STRIPE_PRICE_ID").ok(),
        std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
    ) {
        (Some(secret_key), Some(price_id), Some(webhook_secret)) => Some(StripeConfig {
            secret_key,
            price_id,
            webhook_secret,
        }),
        (None, None, None) => None,
        _ => panic!(
            "Stripe E2E requires STRIPE_SECRET_KEY, STRIPE_PRICE_ID, and STRIPE_WEBHOOK_SECRET"
        ),
    };

    let environment = Environment {
        database_url: format!("sqlite:{database_file}?mode=rwc"),
        port,
        public_url: Some(public_url),
        smtp: None,
        stripe,
    };

    let email_client: Option<Arc<dyn Mailer + Send + Sync>> =
        Some(Arc::new(FakeMailer { path: email_file }));

    build_app(environment, email_client).await;
}

struct FakeMailer {
    path: String,
}

impl Mailer for FakeMailer {
    fn sender(&self) -> &str {
        "e2e@example.com"
    }

    fn send(&self, message: &Message) -> Result<(), AppError> {
        std::fs::write(&self.path, message.formatted())?;
        Ok(())
    }
}
