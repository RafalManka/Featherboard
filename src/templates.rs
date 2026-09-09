use crate::error::AppError;
use crate::login::is_logged_in;
use crate::org::CurrentOrg;
use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use tower_sessions::Session;

pub struct HtmlTemplate<T>(pub T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("template render error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
        }
    }
}

pub struct Layout {
    pub org_slug: String,
    pub is_logged_in: bool,
    pub accent_color: Option<String>,
    pub logo_url: Option<String>,
}

impl Layout {
    pub async fn load(current_org: &CurrentOrg, session: &Session) -> Result<Layout, AppError> {
        Ok(Self {
            org_slug: current_org.slug.clone(),
            is_logged_in: is_logged_in(session).await?,
            accent_color: current_org.accent_color.clone(),
            logo_url: current_org.logo_url.clone(),
        })
    }
}
