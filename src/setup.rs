use crate::AppState;
use crate::email::MagicLinkEmail;
use crate::error::AppError;
use crate::templates::HtmlTemplate;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use hmac::digest::Digest;
use regex::Regex;
use serde::Deserialize;
use sha2::Sha256;
use tower_sessions::Session;
use uuid::Uuid;

pub fn setup_router() -> Router<AppState> {
    Router::new()
        .route("/", get(setup_page))
        .route("/", post(create_org))
        .route("/admin", get(setup_admin_page))
        .route("/admin", post(submit_admin_page))
        .route("/confirm/{token}", get(submit_admin_magic_link))
}

#[derive(Template)]
#[template(path = "pages/setup.html")]
pub struct SetupTemplate {
    name: String,
    name_error: Option<String>,

    slug: String,
    slug_error: Option<String>,
}

pub async fn setup_page() -> Result<HtmlTemplate<SetupTemplate>, AppError> {
    Ok(HtmlTemplate(SetupTemplate {
        name: "".to_string(),
        name_error: None,
        // name_error: Some("Use an organisation name between 2 and 80 characters.".to_string()),
        slug: "".to_string(),
        slug_error: None,
        // slug_error: Some("Use 3–63 lowercase letters or numbers, separated by single hyphens.".to_string()),
    }))
}

#[derive(Deserialize)]
pub struct CreateOrgForm {
    name: String,
    slug: String,
}

pub async fn create_org(
    session: Session,
    Form(form): Form<CreateOrgForm>,
) -> Result<Response, AppError> {
    let name = form.name.trim().to_string();
    let slug = form.slug.trim().to_string();

    let name_error = validate_name(&name)?;
    let slug_error = validate_slug(&slug)?;

    if name_error.is_some() || slug_error.is_some() {
        return Ok(HtmlTemplate(SetupTemplate {
            name,
            name_error,
            slug,
            slug_error,
        })
        .into_response());
    }

    session.insert("setup_org_name", &name).await?;
    session.insert("setup_org_slug", &slug).await?;

    Ok(Redirect::to("/setup/admin").into_response())
}

fn validate_slug(slug: &str) -> Result<Option<String>, AppError> {
    let slug = slug.trim();
    let valid_slug = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")?.is_match(slug);

    if !valid_slug || !(3..=63).contains(&slug.len()) {
        return Ok(Some(
            "Use 3–63 lowercase letters or numbers, separated by single hyphens.".to_string(),
        ));
    }

    Ok(None)
}

fn validate_name(name: &str) -> Result<Option<String>, AppError> {
    let length = name.trim().chars().count();

    if !(2..=80).contains(&length) {
        return Ok(Some(
            "Use an organisation name between 2 and 80 characters.".to_string(),
        ));
    }

    Ok(None)
}

#[derive(Template)]
#[template(path = "pages/setup_admin.html")]
pub struct SetupAdminTemplate {
    email: String,
    email_error: Option<String>,
    success_message: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateAdminForm {
    email: String,
}

pub async fn setup_admin_page(session: Session) -> Result<Response, AppError> {
    let Some(name) = session.get::<String>("setup_org_name").await? else {
        return Ok(Redirect::to("/setup").into_response());
    };

    let Some(slug) = session.get::<String>("setup_org_slug").await? else {
        return Ok(Redirect::to("/setup").into_response());
    };

    if validate_name(&name)?.is_some() || validate_slug(&slug)?.is_some() {
        return Ok(Redirect::to("/setup").into_response());
    }

    Ok(HtmlTemplate(SetupAdminTemplate {
        email: "".to_string(),
        email_error: None,
        success_message: None,
    })
    .into_response())
}

pub async fn submit_admin_page(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CreateAdminForm>,
) -> Result<Response, AppError> {
    let Some(name) = session.get::<String>("setup_org_name").await? else {
        return Ok(Redirect::to("/setup").into_response());
    };

    let Some(slug) = session.get::<String>("setup_org_slug").await? else {
        return Ok(Redirect::to("/setup").into_response());
    };

    if validate_name(&name)?.is_some() || validate_slug(&slug)?.is_some() {
        return Ok(Redirect::to("/setup").into_response());
    }

    let email = form.email.trim().to_string();

    let email_error = validate_email(&email)?;
    if email_error.is_some() {
        return Ok(HtmlTemplate(SetupAdminTemplate {
            email,
            email_error,
            success_message: None,
        })
        .into_response());
    }

    let token = Uuid::new_v4().to_string();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    let sql = r#"
        INSERT INTO pending_organizations (name, slug, email, token_hash)
        VALUES (?, ?, ?, ?)
    "#;

    sqlx::query(sql)
        .bind(&name)
        .bind(&slug)
        .bind(&email)
        .bind(&token_hash)
        .execute(&state.db)
        .await?;

    let Some(email_client) = state.email_client.as_ref() else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            "Email setup is unavailable",
        )
            .into_response());
    };

    let public_url = state.environment.public_url.as_deref().unwrap_or("");
    let link = format!("{public_url}/setup/confirm/{token}");

    email_client
        .send_magic_link(MagicLinkEmail {
            email: email.clone(),
            token,
            link,
        })
        .await?;

    Ok(HtmlTemplate(SetupAdminTemplate {
        email,
        email_error: None,
        success_message: Some("please check your email".to_string()),
    })
    .into_response())
}

fn validate_email(email: &str) -> Result<Option<String>, AppError> {
    const EMAIL_ADDRESS_REGEX: &str = r#"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"#;

    if !Regex::new(EMAIL_ADDRESS_REGEX)?.is_match(email) {
        return Ok(Some("Email is not valid.".to_string()));
    }

    Ok(None)
}

#[derive(Deserialize)]
pub struct TokenParam {
    token: String,
}

pub async fn submit_admin_magic_link(
    State(state): State<AppState>,
    session: Session,
    Path(token_param): Path<TokenParam>,
) -> Result<Response, AppError> {
    let hash = hex::encode(Sha256::digest(token_param.token.as_bytes()));

    let mut tx = state.db.begin().await?;

    let sql = r#"
        DELETE FROM pending_organizations
        WHERE token_hash = ? AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        RETURNING name, slug, email
    "#;

    let result: Option<(String, String, String)> = sqlx::query_as(sql)
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(result) = result else {
        return Ok(Redirect::to("/").into_response());
    };

    let sql = r#"
        INSERT INTO organizations (name, slug)
        VALUES (?, ?)
        RETURNING id
    "#;

    let org_id: i64 = sqlx::query_scalar(sql)
        .bind(&result.0)
        .bind(&result.1)
        .fetch_one(&mut *tx)
        .await?;

    let email = result.2.to_lowercase();

    let sql = r#"
        INSERT INTO users (email, name, org_id, is_admin, provider, provider_user_id)
        VALUES (?, ?, ?, 1, 'email', ?)
        RETURNING id
    "#;

    let user_id: i64 = sqlx::query_scalar(sql)
        .bind(&email)
        .bind(&email)
        .bind(org_id)
        .bind(&email)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;
    session.insert("user_id", user_id).await?;

    Ok(Redirect::to(&format!("/{}", &result.1)).into_response())
}
