use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

use featherboard::environment::{Environment, StripeConfig};

pub async fn test_db() -> Pool<Sqlite> {
    let db = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");
    db
}

pub fn test_environment() -> Environment {
    Environment {
        database_url: "sqlite::memory:".to_string(),
        port: 3000,
        public_url: None,
        smtp: None,
        stripe: None,
    }
}

pub fn test_stripe_environment(secret: &str) -> Environment {
    Environment {
        public_url: Some("http://localhost:3000".to_string()),
        stripe: Some(StripeConfig {
            secret_key: "sk_test_dummy".to_string(),
            price_id: "price_dummy".to_string(),
            webhook_secret: secret.to_string(),
        }),
        ..test_environment()
    }
}
