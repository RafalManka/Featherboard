use featherboard::email::{EmailClient, Mailer};
use featherboard::environment::Environment;
use featherboard::server::build_app;
//noinspection RsUnresolvedPath
use std::io::IsTerminal;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
//noinspection RsReferenceIsNotPublic
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let environment = Environment::load().expect("Failed to load environment");

    let email_client: Option<Arc<dyn Mailer + Send + Sync>> = environment
        .smtp
        .clone()
        .map(EmailClient::new)
        .map(|client| Arc::new(client) as Arc<dyn Mailer + Send + Sync>);

    build_app(environment, email_client).await;
}
