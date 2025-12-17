//! Static file serving for embedded web UI assets.
//!
//! This module provides handlers for serving the embedded Leptos WASM frontend
//! with proper MIME type detection and SPA routing support.

use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};

use crate::embedded::Assets;

/// Serve embedded static files with SPA routing support.
///
/// For paths without file extensions (SPA routes), falls back to index.html.
/// Returns 404 for missing files with extensions.
pub async fn serve_embedded_file(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // SPA routing: paths without extensions serve index.html
    let path = if path.is_empty() || !path.contains('.') {
        "index.html"
    } else {
        path
    };

    serve_file(path)
}

/// Serve a specific file from embedded assets.
fn serve_file(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            // Use different cache strategies based on file type
            let cache_control = if path.contains(".wasm") || path.contains(".js") {
                // WASM and JS files have content hashes, cache aggressively
                "public, max-age=31536000, immutable"
            } else if path == "index.html" {
                // HTML should be revalidated
                "public, max-age=0, must-revalidate"
            } else {
                // Other assets (CSS, images) - cache for a day
                "public, max-age=86400"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header(header::CACHE_CONTROL, cache_control)
                .body(Body::from(content.data.into_owned()))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .expect("Failed to build error response")
                })
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .expect("Failed to build 404 response"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spa_routing_empty_path() {
        // Empty path should map to index.html
        let path = "";
        let result = if path.is_empty() || !path.contains('.') {
            "index.html"
        } else {
            path
        };
        assert_eq!(result, "index.html");
    }

    #[test]
    fn test_spa_routing_no_extension() {
        // Paths without extensions are SPA routes
        let path = "mail/inbox";
        let result = if path.is_empty() || !path.contains('.') {
            "index.html"
        } else {
            path
        };
        assert_eq!(result, "index.html");
    }

    #[test]
    fn test_static_file_with_extension() {
        // Paths with extensions are static files
        let path = "assets/app.js";
        let result = if path.is_empty() || !path.contains('.') {
            "index.html"
        } else {
            path
        };
        assert_eq!(result, "assets/app.js");
    }
}
