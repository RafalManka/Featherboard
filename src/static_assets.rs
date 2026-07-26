use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

pub async fn serve(Path(path): Path<String>) -> Response {
    match Assets::get(&path) {
        Some(file) => ([(header::CONTENT_TYPE, file.metadata.mimetype())], file.data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
