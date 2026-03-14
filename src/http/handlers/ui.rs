//! Static file serving for embedded UI.

use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "src/ui/"]
struct Assets;

/// Serve the main index page.
pub async fn index() -> impl IntoResponse {
    match Assets::get("index.html") {
        Some(content) => Html(String::from_utf8_lossy(&content.data).to_string()).into_response(),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

/// Serve static files (JS, CSS, etc.).
pub async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
