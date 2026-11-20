use featherboard::email::Mailer;
use featherboard::environment::Environment;
use featherboard::error::AppError;
use featherboard::server::build_app;
use lettre::Message;
use std::sync::Arc;

const EMAIL_FILE: &str = "/private/tmp/featherboard-e2e-email.eml";
const DATABASE_FILE: &str = "/private/tmp/featherboard-e2e.db";

#[tokio::main]
async fn main() {
    let _ = std::fs::remove_file(EMAIL_FILE);
    let _ = std::fs::remove_file(DATABASE_FILE);

    let environment = Environment {
        database_url: format!("sqlite:{DATABASE_FILE}?mode=rwc"),
        port: 3101,
        public_url: Some("http://127.0.0.1:3101".to_string()),
        smtp: None,
        stripe: None,
    };

    let email_client: Option<Arc<dyn Mailer + Send + Sync>> = Some(Arc::new(FakeMailer));

    build_app(environment, email_client).await;
}

struct FakeMailer;

impl Mailer for FakeMailer {
    fn sender(&self) -> &str {
        "e2e@example.com"
    }

    fn send(&self, message: &Message) -> Result<(), AppError> {
        std::fs::write(EMAIL_FILE, message.formatted())?;
        Ok(())
    }
}
