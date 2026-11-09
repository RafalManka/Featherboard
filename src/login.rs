use crate::AppState;
use crate::email::{EmailNotifier, MagicLinkEmail};
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tower_sessions::Session;
use uuid::Uuid;

pub fn login_router() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_form))
        .route("/login", post(login_form_submit))
        .route("/login/check-email", get(check_email_form))
        .route("/login/check-email/{token}", get(confirm_magic_link))
        .route("/logout", get(logout))
}

#[derive(Template)]
#[template(path = "pages/admin_login.html")]
pub struct AdminLoginTemplate {
    layout: Layout,
    github_enabled: bool,
}

pub async fn is_logged_in(
    db: &SqlitePool,
    session: &Session,
    org_id: i64,
) -> Result<bool, AppError> {
    let user_id = session.get::<i64>("user_id").await?;
    let sql = r#"
        SELECT EXISTS (SELECT id FROM users WHERE id = ? AND org_id = ?)
    "#;

    let exists = sqlx::query_scalar(sql)
        .bind(user_id)
        .bind(org_id)
        .fetch_one(db)
        .await?;

    Ok(exists)
}

async fn login_form(
    State(state): State<AppState>,
    session: Session,
    current_org: CurrentOrg,
) -> Result<Response, AppError> {
    let github_enabled =
        current_org.github_client_secret.is_some() && current_org.github_client_id.is_some();

    Ok(HtmlTemplate(AdminLoginTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        github_enabled,
    })
    .into_response())
}

async fn logout(session: Session, current_org: CurrentOrg) -> Result<Response, AppError> {
    session.remove::<i64>("user_id").await?;
    Ok(current_org.redirect("").into_response())
}

#[derive(Deserialize)]
pub struct EmailLoginForm {
    email: String,
}

async fn login_form_submit(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    form: Form<EmailLoginForm>,
) -> Result<Response, AppError> {
    if !is_valid_email(&form.email) {
        return Ok((StatusCode::BAD_REQUEST, "Invalid email address").into_response());
    }

    let Some(email_client) = state.email_client else {
        return Ok((StatusCode::SERVICE_UNAVAILABLE, "SMTP is unavailable").into_response());
    };

    let token = Uuid::new_v4().to_string();
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    let email = form.email.to_lowercase();

    let sql = r#"
        INSERT INTO magic_links (org_id, email, token_hash)
        VALUES (?, ?, ?)
        ON CONFLICT (org_id, email, token_hash) DO UPDATE SET
            email = excluded.email,
            token_hash = excluded.token_hash,
            created_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+5 minutes')
        WHERE org_id = ?
    "#;

    sqlx::query(sql)
        .bind(current_org.id)
        .bind(&email)
        .bind(&hash)
        .bind(current_org.id)
        .execute(&state.db)
        .await?;

    let link = current_org.path(&format!("/login/check-email/{}", &token));
    EmailNotifier::new(&state.db, &current_org, &email_client)
        .magic_link(MagicLinkEmail { email, token, link })
        .await?;

    Ok(current_org.redirect("/login/check-email").into_response())
}

const EMAIL_ADDRESS_REGEX: &str = r#"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"#;

fn is_valid_email(email: &str) -> bool {
    Regex::new(EMAIL_ADDRESS_REGEX).unwrap().is_match(email)
}

#[derive(Template)]
#[template(path = "pages/check_email.html")]
pub struct CheckEmailTemplate {
    layout: Layout,
}

#[derive(Deserialize)]
struct IdParam {
    token: String,
}

#[derive(Template)]
#[template(path = "pages/invalid_token.html")]
pub struct InvalidTokenTemplate {
    layout: Layout,
}

async fn confirm_magic_link(
    State(state): State<AppState>,
    session: Session,
    current_org: CurrentOrg,
    Path(token_param): Path<IdParam>,
) -> Result<Response, AppError> {
    let hash = hex::encode(Sha256::digest(token_param.token.as_bytes()));

    let sql = r#"
        DELETE FROM magic_links
        WHERE org_id = ? AND token_hash = ? AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        RETURNING email
    "#;

    let email = sqlx::query_scalar::<_, String>(sql)
        .bind(current_org.id)
        .bind(hash)
        .fetch_optional(&state.db)
        .await?;

    let Some(email) = email else {
        return Ok(HtmlTemplate(InvalidTokenTemplate {
            layout: Layout::load(&state.db, &current_org, &session).await?,
        })
        .into_response());
    };

    let sql = r#"
        INSERT INTO users (org_id, provider, provider_user_id, email, name)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (org_id, provider, provider_user_id) DO UPDATE SET
            email = excluded.email,
            name = excluded.name
        RETURNING id
    "#;

    let email = email.to_lowercase();
    let user_id: i64 = sqlx::query_scalar(sql)
        .bind(current_org.id)
        .bind("email")
        .bind(&email)
        .bind(&email)
        .bind(&email)
        .fetch_one(&state.db)
        .await?;

    session.insert("user_id", user_id).await?;

    Ok(current_org.redirect("").into_response())
}

async fn check_email_form(
    State(state): State<AppState>,
    session: Session,
    current_org: CurrentOrg,
) -> Result<Response, AppError> {
    Ok(HtmlTemplate(CheckEmailTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
    })
    .into_response())
}

#[cfg(test)]
mod test {
    use crate::login::is_valid_email;

    #[test]
    fn validate_email_address() {
        assert!(is_valid_email("example@email.com"));
        assert!(!is_valid_email("@email.com"));
        assert!(!is_valid_email("invalid@email.x"));
    }

    // TODO: test email sending?
}
