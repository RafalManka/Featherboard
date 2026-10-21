use crate::AppState;
use crate::admin::is_admin;
use crate::error::AppError;
use crate::org::CurrentOrg;
use crate::subscription::{SubscriptionView, get_subscription_view};
use crate::templates::{HtmlTemplate, Layout};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Router};
use serde::Deserialize;
use tower_sessions::Session;

pub fn settings_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_settings))
        .route("/branding", post(set_branding))
        .route("/github", post(set_github))
}

#[derive(Template)]
#[template(path = "pages/settings.html")]
struct SettingsTemplate {
    layout: Layout,
    subscription: SubscriptionView,
    github: GithubView,
}

async fn get_settings(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
) -> Result<Response, AppError> {
    if !is_admin(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    Ok(HtmlTemplate(SettingsTemplate {
        layout: Layout::load(&state.db, &current_org, &session).await?,
        subscription: get_subscription_view(&state.db, current_org.id).await?,
        github: load_github_view(&current_org),
    })
    .into_response())
}

#[derive(Deserialize)]
pub struct BrandingForm {
    accent_color: String,
    logo_url: String,
}

async fn set_branding(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<BrandingForm>,
) -> Result<Response, AppError> {
    if !is_admin(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let sql = r#"
        UPDATE organizations
        SET
            accent_color=?,
            logo_url=?
        WHERE id = ?
    "#;

    sqlx::query(sql)
        .bind(Some(form.accent_color).filter(|s| !s.is_empty()))
        .bind(Some(form.logo_url).filter(|s| !s.is_empty()))
        .bind(current_org.id)
        .execute(&state.db)
        .await?;

    Ok(current_org.redirect("/settings").into_response())
}

/// Stands in for a stored secret on GET (so the real value never reaches
/// the browser) and, on POST, signals "admin didn't touch this field" —
/// see set_github below. Not derived from the real secret in any way
/// (no partial reveal, no length hint).
const SECRET_PLACEHOLDER: &str = "********";

fn load_github_view(current_org: &CurrentOrg) -> GithubView {
    GithubView {
        github_client_id: current_org.github_client_id.clone().unwrap_or_default(),
        github_client_secret: if current_org.github_client_secret.is_some() {
            SECRET_PLACEHOLDER.to_string()
        } else {
            String::new()
        },
    }
}

pub struct GithubView {
    github_client_id: String,
    github_client_secret: String,
}

#[derive(Deserialize)]
pub struct GithubForm {
    github_client_id: String,
    github_client_secret: String,
}

async fn set_github(
    State(state): State<AppState>,
    current_org: CurrentOrg,
    session: Session,
    Form(form): Form<GithubForm>,
) -> Result<Response, AppError> {
    if !is_admin(&state.db, &session, current_org.id).await? {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    sqlx::query("UPDATE organizations SET github_client_id = ? WHERE id = ?")
        .bind(Some(form.github_client_id).filter(|s| !s.is_empty()))
        .bind(current_org.id)
        .execute(&state.db)
        .await?;

    // The placeholder means "admin didn't touch this field" — leave the
    // stored secret alone entirely rather than overwriting it with the
    // literal placeholder text. Anything else (including empty, which
    // clears it to NULL) is a deliberate change.
    if form.github_client_secret != SECRET_PLACEHOLDER {
        sqlx::query("UPDATE organizations SET github_client_secret = ? WHERE id = ?")
            .bind(Some(form.github_client_secret).filter(|s| !s.is_empty()))
            .bind(current_org.id)
            .execute(&state.db)
            .await?;
    }

    Ok(current_org.redirect("/settings").into_response())
}
