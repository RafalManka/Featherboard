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
}

#[derive(Template)]
#[template(path = "pages/settings.html")]
struct SettingsTemplate {
    layout: Layout,
    subscription: SubscriptionView,
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
