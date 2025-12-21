use std::sync::Arc;

use axum::{
    Router,
    http::{StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;
use tracing::debug;

use crate::peercast::Store;

#[derive(Embed)]
#[folder = "client/dist/"]
struct Assets;
static INDEX_HTML: &str = "index.html";

pub fn build_router(_store: &Arc<Store>) -> axum::Router {
    Router::new().fallback(static_handler)
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    debug!("Static file request: {}", uri.path());
    let path = uri.path().trim_start_matches('/');

    match path {
        "" => {
            return index_html().await;
        }
        _ => {}
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }
            index_html().await
        }
    }
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_index_html() {
        let response = index_html().await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_not_found() {
        let response = not_found().await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_static_handler_index() {
        let uri: Uri = "/".parse().unwrap();
        let response = static_handler(uri).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_static_handler_not_found() {
        let uri: Uri = "/nonexistentfile.xyz".parse().unwrap();
        let response = static_handler(uri).await.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_correct_url_not_found() {
        let uri: Uri = "/ui".parse().unwrap();
        let response = static_handler(uri).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_correct_url_slash_not_found() {
        let uri: Uri = "/ui/".parse().unwrap();
        let response = static_handler(uri).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
