use crate::error::AppError;
use crate::org::CurrentOrg;
use askama::Template;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls::Required;
use lettre::transport::smtp::client::TlsParametersBuilder;
use lettre::{Message, SmtpTransport, Transport};
use sqlx::SqlitePool;
use std::env;

#[derive(Clone)]
pub struct EmailClient {
    pub sender: String,
    password: String,
    smtp: String,
    port: u16,
}

impl EmailClient {
    pub fn load() -> Option<Self> {
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

        Some(EmailClient {
            sender,
            password,
            smtp,
            port,
        })
    }
    pub fn send_email(&self, message: &Message) -> Result<(), Box<lettre::transport::smtp::Error>> {
        SmtpTransport::relay(&self.smtp)?
            .port(self.port)
            .credentials(Credentials::new(self.sender.clone(), self.password.clone()))
            .tls(Required(
                TlsParametersBuilder::new(self.smtp.clone()).build()?,
            ))
            .build()
            .send(&message)?;
        Ok(())
    }
}

#[derive(Template)]
#[template(path = "emails/payment_success.html")]
pub struct PaymentSuccessEmail {
    pub subscription_url: String,
}

pub async fn notify_payment_success(
    db: &SqlitePool,
    current_org: &CurrentOrg,
    email_client: &EmailClient,
    template: PaymentSuccessEmail,
) -> Result<(), AppError> {
    let sql = r#"
           SELECT email
           FROM users
           WHERE is_admin = 1 AND email IS NOT NULL AND org_id = ?
        "#;

    let emails: Vec<String> = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .fetch_all(db)
        .await
        .unwrap_or(vec![]);

    for email in emails {
        let message = Message::builder()
            .from(Mailbox::new(
                Some(current_org.slug.clone()),
                email_client.sender.parse()?,
            ))
            .to(Mailbox::new(None, email.parse()?))
            .subject("Your subscription has been paid")
            .header(ContentType::TEXT_HTML)
            .body(template.render()?)?;

        _ = email_client.send_email(&message);
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "emails/payment_failed.html")]
pub struct PaymentFailedEmail {
    pub payment_url: String,
}

pub async fn notify_payment_failed(
    db: &SqlitePool,
    current_org: &CurrentOrg,
    email_client: &EmailClient,
    template: PaymentFailedEmail,
) -> Result<(), AppError> {
    let sql = r#"
           SELECT email
           FROM users
           WHERE is_admin = 1 AND email IS NOT NULL AND org_id = ?
        "#;

    let emails: Vec<String> = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .fetch_all(db)
        .await
        .unwrap_or(vec![]);

    for email in emails {
        let message = Message::builder()
            .from(Mailbox::new(
                Some(current_org.slug.clone()),
                email_client.sender.parse()?,
            ))
            .to(Mailbox::new(None, email.parse()?))
            .subject("Payment failed for your Featherboard subscription")
            .header(ContentType::TEXT_HTML)
            .body(template.render()?)?;

        _ = email_client.send_email(&message);
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "emails/comment_created.html")]
pub struct NewCommentEmail {
    pub idea_title: String,
    pub author_name: String,
    pub comment_body: String,
    pub idea_url: String,
}
pub async fn notify_comment_created(
    db: &SqlitePool,
    current_org: &CurrentOrg,
    email_client: &EmailClient,
    user_id: Option<i64>,
    idea_id: i64,
    template: NewCommentEmail,
) -> Result<(), AppError> {
    let sql = r#"
           SELECT DISTINCT u.email
           FROM users u
           WHERE u.org_id = ?
            AND u.id IN (
                SELECT user_id FROM ideas
                WHERE id = ? AND org_id = ? AND user_id IS NOT NULL
                UNION
                SELECT user_id FROM votes
                WHERE idea_id = ? AND user_id IS NOT NULL
                UNION
                SELECT id FROM users
                WHERE is_admin = 1 AND org_id = ?
            )
            AND u.id != ?
        "#;

    let emails: Vec<String> = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(user_id.unwrap_or(-1))
        .fetch_all(db)
        .await
        .unwrap_or(vec![]);

    for email in emails {
        let message = Message::builder()
            .from(Mailbox::new(
                Some(current_org.slug.clone()),
                email_client.sender.parse()?,
            ))
            .to(Mailbox::new(None, email.parse()?))
            .subject(format!("New comment on: {}", template.idea_title))
            .header(ContentType::TEXT_HTML)
            .body(template.render()?)?;

        _ = email_client.send_email(&message);
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "emails/status_changed.html")]
pub struct StatusChangedEmail {
    pub idea_title: String,
    pub new_status: String,
    pub idea_url: String,
}
pub async fn notify_status_changed(
    user_id: Option<i64>,
    idea_id: i64,

    db: &SqlitePool,
    current_org: &CurrentOrg,
    email_client: &EmailClient,
    template: StatusChangedEmail,
) -> Result<(), AppError> {
    let sql = r#"
           SELECT DISTINCT u.email
           FROM users u
           WHERE u.org_id = ?
            AND u.id IN (
                SELECT user_id FROM ideas
                WHERE id = ? AND org_id = ? AND user_id IS NOT NULL
                UNION
                SELECT user_id FROM votes
                WHERE idea_id = ? AND user_id IS NOT NULL
                UNION
                SELECT id FROM users
                WHERE is_admin = 1 AND org_id = ?
            )
            AND u.id != ?
        "#;

    let emails: Vec<String> = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(user_id.unwrap_or(-1))
        .fetch_all(db)
        .await
        .unwrap_or(vec![]);

    for email in emails {
        let message = Message::builder()
            .from(Mailbox::new(
                Some(current_org.slug.clone()),
                email_client.sender.parse()?,
            ))
            .to(Mailbox::new(None, email.parse()?))
            .subject(format!(
                "Status changed to {}: {}",
                template.new_status, template.idea_title
            ))
            .header(ContentType::TEXT_HTML)
            .body(template.render()?)?;

        _ = email_client.send_email(&message);
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "emails/changelog_created.html")]
pub struct ChangelogCreatedEmail {
    pub changelog_title: String,
    pub changelog_body: String,
    pub changelog_url: String,
}
pub async fn notify_changelog_created(
    db: &SqlitePool,
    current_org: &CurrentOrg,
    email_client: &EmailClient,
    template: ChangelogCreatedEmail,
) -> Result<(), AppError> {
    let sql = r#"
           SELECT email
           FROM users
           WHERE is_admin = 1 AND email IS NOT NULL AND org_id = ?
        "#;

    let emails: Vec<String> = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .fetch_all(db)
        .await
        .unwrap_or(vec![]);

    for email in emails {
        let message = Message::builder()
            .from(Mailbox::new(
                Some(current_org.slug.clone()),
                email_client.sender.parse()?,
            ))
            .to(Mailbox::new(None, email.parse()?))
            .subject(format!("Changelog created: {}", template.changelog_title))
            .header(ContentType::TEXT_HTML)
            .body(template.render()?)?;

        _ = email_client.send_email(&message);
    }

    Ok(())
}
