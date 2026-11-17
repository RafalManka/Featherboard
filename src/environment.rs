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
    pub fn load() -> Result<Environment, ConfigError> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:featherboard.db?mode=rwc".to_string());

        let port = env::var("PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(3000);

        let public_url = env::var("PUBLIC_URL")
            .ok()
            .map(|url| url.trim().trim_end_matches('/').to_string())
            .filter(|url| !url.is_empty());

        let smtp = SmtpConfig::load()?;
        let stripe = StripeConfig::load()?;

        if smtp.is_some() && public_url.is_none() {
            panic!("configuration error, you need to have public URL in order to use SMTP")
        }

        if stripe.is_some() && public_url.is_none() {
            panic!("configuration error, you need to have public URL in order to use Stripe")
        }

        Ok(Self {
            database_url,
            port,
            public_url,
            smtp,
            stripe,
        })
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
    fn load() -> Result<Option<Self>, ConfigError> {
        let configured = [
            "STRIPE_SECRET_KEY",
            "STRIPE_PRICE_ID",
            "STRIPE_WEBHOOK_SECRET",
        ]
        .iter()
        .any(|name| env::var_os(name).is_some());

        if !configured {
            return Ok(None);
        }

        Ok(Some(Self {
            secret_key: required_env("STRIPE_SECRET_KEY")?,
            price_id: required_env("STRIPE_PRICE_ID")?,
            webhook_secret: required_env("STRIPE_WEBHOOK_SECRET")?,
        }))
    }
}

impl SmtpConfig {
    fn load() -> Result<Option<Self>, ConfigError> {
        let configured = ["SMTP_USER", "SMTP_PASSWORD", "SMTP_ADDRESS", "SMTP_PORT"]
            .iter()
            .any(|name| env::var_os(name).is_some());

        if !configured {
            return Ok(None);
        }

        Ok(Some(Self {
            sender: required_env("SMTP_USER")?,
            password: required_env("SMTP_PASSWORD")?,
            smtp: required_env("SMTP_ADDRESS")?,
            port: required_env("SMTP_PORT")?
                .parse::<u16>()
                .map_err(|_| ConfigError::InvalidPort("SMTP_PORT".to_string()))?,
        }))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Missing(String),
    Empty(String),
    InvalidPort(String),
}

fn required_env(name: &str) -> Result<String, ConfigError> {
    let value = env::var(name).map_err(|_| ConfigError::Missing(name.to_string()))?;

    let value = value.trim().to_string();

    if value.is_empty() {
        return Err(ConfigError::Empty(name.to_string()));
    }

    Ok(value)
}
