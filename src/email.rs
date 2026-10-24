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

pub trait Mailer {
    fn sender(&self) -> &str;
    fn send(&self, message: &Message) -> Result<(), ()>;
}

impl Mailer for EmailClient {
    fn sender(&self) -> &str {
        &self.sender
    }

    fn send(&self, message: &Message) -> Result<(), ()> {
        self.send_email(message).map_err(|_| ())
    }
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

#[derive(Template)]
#[template(path = "emails/status_changed.html")]
pub struct StatusChangedEmail {
    pub idea_title: String,
    pub new_status: String,
    pub idea_url: String,
}

async fn notification_recipient_emails(
    db: &SqlitePool,
    current_org: &CurrentOrg,
    user_id: Option<i64>,
    idea_id: i64,
) -> Vec<String> {
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

    sqlx::query_scalar(sql)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(idea_id)
        .bind(current_org.id)
        .bind(user_id.unwrap_or(-1))
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub struct EmailNotifier<'a, M: Mailer + ?Sized> {
    db: &'a SqlitePool,
    current_org: &'a CurrentOrg,
    email_client: &'a M,
}

impl<'a, M: Mailer + ?Sized> EmailNotifier<'a, M> {
    pub fn new(db: &'a SqlitePool, current_org: &'a CurrentOrg, email_client: &'a M) -> Self {
        Self {
            db,
            current_org,
            email_client,
        }
    }

    pub async fn comment_created(
        &self,
        user_id: Option<i64>,
        idea_id: i64,
        template: NewCommentEmail,
    ) -> Result<(), AppError> {
        let emails =
            notification_recipient_emails(self.db, self.current_org, user_id, idea_id).await;

        for email in emails {
            let message = Message::builder()
                .from(Mailbox::new(
                    Some(self.current_org.slug.clone()),
                    self.email_client.sender().parse()?,
                ))
                .to(Mailbox::new(None, email.parse()?))
                .subject(format!("New comment on: {}", template.idea_title))
                .header(ContentType::TEXT_HTML)
                .body(template.render()?)?;

            _ = self.email_client.send(&message);
        }

        Ok(())
    }

    pub async fn status_changed(
        &self,
        user_id: Option<i64>,
        idea_id: i64,
        template: StatusChangedEmail,
    ) -> Result<(), AppError> {
        let emails =
            notification_recipient_emails(self.db, self.current_org, user_id, idea_id).await;

        for email in emails {
            let message = Message::builder()
                .from(Mailbox::new(
                    Some(self.current_org.slug.clone()),
                    self.email_client.sender().parse()?,
                ))
                .to(Mailbox::new(None, email.parse()?))
                .subject(format!(
                    "Status changed to {}: {}",
                    template.new_status, template.idea_title
                ))
                .header(ContentType::TEXT_HTML)
                .body(template.render()?)?;

            _ = self.email_client.send(&message);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmailNotifier, Mailer, NewCommentEmail, StatusChangedEmail, notification_recipient_emails,
    };
    use crate::org::CurrentOrg;
    use lettre::Message;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Mutex;

    fn current_org(id: i64) -> CurrentOrg {
        CurrentOrg {
            id,
            slug: format!("org-{id}"),
            github_client_secret: None,
            github_client_id: None,
            accent_color: None,
            logo_url: None,
        }
    }

    async fn test_db() -> sqlx::SqlitePool {
        let db = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                org_id INTEGER NOT NULL,
                email TEXT NOT NULL,
                is_admin INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE ideas (
                id INTEGER PRIMARY KEY,
                org_id INTEGER NOT NULL,
                user_id INTEGER
            )",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE votes (
                idea_id INTEGER NOT NULL,
                user_id INTEGER
            )",
        )
        .execute(&db)
        .await
        .unwrap();

        db
    }

    //noinspection SqlResolve
    //noinspection SqlInsertValues
    #[tokio::test]
    async fn notification_recipients_include_creator_voters_and_admins_once() {
        let db = test_db().await;
        sqlx::query(
            "INSERT INTO users (id, org_id, email, is_admin) VALUES
                (1, 1, 'creator@example.com', 0),
                (2, 1, 'voter@example.com', 0),
                (3, 1, 'admin@example.com', 1),
                (4, 1, 'creator@example.com', 0),
                (5, 2, 'other-org@example.com', 1)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ideas (id, org_id, user_id) VALUES (10, 1, 1)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO votes (idea_id, user_id) VALUES (10, 2), (10, 4)")
            .execute(&db)
            .await
            .unwrap();

        let mut emails = notification_recipient_emails(&db, &current_org(1), None, 10).await;
        emails.sort();

        assert_eq!(
            emails,
            vec![
                "admin@example.com".to_string(),
                "creator@example.com".to_string(),
                "voter@example.com".to_string(),
            ]
        );
    }

    //noinspection SqlResolve
    //noinspection SqlInsertValues
    #[tokio::test]
    async fn notification_recipients_exclude_the_triggering_user() {
        let db = test_db().await;
        sqlx::query(
            "INSERT INTO users (id, org_id, email, is_admin) VALUES
                (1, 1, 'creator@example.com', 0),
                (2, 1, 'voter@example.com', 1),
                (3, 1, 'admin@example.com', 1)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ideas (id, org_id, user_id) VALUES (10, 1, 1)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO votes (idea_id, user_id) VALUES (10, 2)")
            .execute(&db)
            .await
            .unwrap();

        let mut emails = notification_recipient_emails(&db, &current_org(1), Some(2), 10).await;
        emails.sort();

        assert_eq!(
            emails,
            vec![
                "admin@example.com".to_string(),
                "creator@example.com".to_string()
            ]
        );
    }

    struct FakeEmailer {
        recipients: Mutex<Vec<String>>,
    }

    impl FakeEmailer {
        fn new() -> Self {
            Self {
                recipients: Mutex::new(Vec::new()),
            }
        }

        fn recipients(&self) -> Vec<String> {
            let mut recipients = self.recipients.lock().unwrap().clone();
            recipients.sort();
            recipients
        }
    }

    impl Mailer for FakeEmailer {
        fn sender(&self) -> &str {
            "foo@example.com"
        }

        fn send(&self, message: &Message) -> Result<(), ()> {
            let mut recipients = self.recipients.lock().unwrap();

            recipients.extend(message.envelope().to().iter().map(ToString::to_string));

            Ok(())
        }
    }

    //noinspection SqlResolve
    //noinspection SqlInsertValues
    #[tokio::test]
    async fn notifier_sends_to_recipients_except_the_triggering_user() {
        let email_client = FakeEmailer::new();

        let db = test_db().await;
        sqlx::query(
            "INSERT INTO users (id, org_id, email, is_admin) VALUES
                (1, 1, 'creator@example.com', 0),
                (2, 1, 'voter@example.com', 1),
                (3, 1, 'admin@example.com', 1)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ideas (id, org_id, user_id) VALUES (10, 1, 1)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO votes (idea_id, user_id) VALUES (10, 2)")
            .execute(&db)
            .await
            .unwrap();

        EmailNotifier::new(&db, &current_org(1), &email_client)
            .comment_created(
                Some(2),
                10,
                NewCommentEmail {
                    idea_title: "Test idea".into(),
                    author_name: "Test user".into(),
                    comment_body: "Test comment".into(),
                    idea_url: "/default/ideas/10".into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(email_client.sender(), "foo@example.com");
        assert_eq!(
            email_client.recipients(),
            vec!["admin@example.com", "creator@example.com"]
        );
    }

    //noinspection SqlResolve
    //noinspection SqlInsertValues
    #[tokio::test]
    async fn status_notifier_sends_to_recipients_except_the_triggering_admin() {
        let email_client = FakeEmailer::new();

        let db = test_db().await;
        sqlx::query(
            "INSERT INTO users (id, org_id, email, is_admin) VALUES
                (1, 1, 'creator@example.com', 0),
                (2, 1, 'admin@example.com', 1),
                (3, 1, 'other-admin@example.com', 1)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ideas (id, org_id, user_id) VALUES (10, 1, 1)")
            .execute(&db)
            .await
            .unwrap();

        EmailNotifier::new(&db, &current_org(1), &email_client)
            .status_changed(
                Some(2),
                10,
                StatusChangedEmail {
                    idea_title: "Test idea".into(),
                    new_status: "Planned".into(),
                    idea_url: "/default/ideas/10".into(),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            email_client.recipients(),
            vec!["creator@example.com", "other-admin@example.com"]
        );
    }
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
