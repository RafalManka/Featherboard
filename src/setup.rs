use crate::AppState;
use crate::error::AppError;
use crate::templates::HtmlTemplate;
use askama::Template;
use axum::Router;
use axum::routing::get;

pub fn setup_router() -> Router<AppState> {
    Router::new().route("/", get(setup_page))
}

#[derive(Template)]
#[template(path = "pages/setup.html")]
pub struct SetupTemplate {}

pub async fn setup_page() -> Result<HtmlTemplate<SetupTemplate>, AppError> {
    Ok(HtmlTemplate(SetupTemplate {}))
}
