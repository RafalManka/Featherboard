use std::env;

#[derive(Clone)]
pub struct Environment {
    pub database_url: String,
    pub port: u16,
    pub public_url: Option<String>,
    pub smtp: Option<SmtpConfig>,
    pub stripe: Option<StripeConfig>,
}

impl Environment {
    pub fn load() -> Environment {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:featherboard.db?mode=rwc".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|port| port.parse().ok())
                .unwrap_or(3000),
            public_url: env::var("PUBLIC_URL")
                .ok()
                .map(|url| url.trim_end_matches('/').to_string()),
            smtp: SmtpConfig::load(),
            stripe: StripeConfig::load(),
        }
    }
}

#[derive(Clone)]
pub struct SmtpConfig {
    pub sender: String,
    pub password: String,
    pub smtp: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub price_id: String,
    pub webhook_secret: String,
}

impl StripeConfig {
    fn load() -> Option<Self> {
        Some(Self {
            secret_key: env::var("STRIPE_SECRET_KEY").ok()?,
            price_id: env::var("STRIPE_PRICE_ID").ok()?,
            webhook_secret: env::var("STRIPE_WEBHOOK_SECRET").ok()?,
        })
    }
}

impl SmtpConfig {
    fn load() -> Option<Self> {
        let Some(sender) = env::var("SMTP_USER").ok() else {
            return None;
        };
        let Some(password) = env::var("SMTP_PASSWORD").ok() else {
            return None;
        };
        let Some(smtp) = env::var("SMTP_ADDRESS").ok() else {
            return None;
        };
        let Some(port) = env::var("SMTP_PORT").ok() else {
            return None;
        };

        let Some(port) = port.parse().ok() else {
            return None;
        };

        Some(Self {
            sender,
            password,
            smtp,
            port,
        })
    }
}
