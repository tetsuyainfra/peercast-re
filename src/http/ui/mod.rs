use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Html,
    routing::{get, Route},
    Router,
};
use axum_core::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;
use rust_embed::RustEmbed;
use serde::de::IntoDeserializer;
use tracing::{debug, info, trace};
use url::Url;

use crate::{config::Config, http::AppState};

use super::MyConnectInfo;
#[cfg(debug_assertions)]
use super::UiProxyMode;

static INDEX_HTML: &str = "index.html";
static VITE_URL: &str = "http://127.0.0.1:5173/ui/";

pub struct Ui;

impl Ui {
    /// /ui以下を受け持つ
    ///
    /// Router::new()
    /// .route("/ui", get(|| async { Redirect::permanent("/ui/") }))
    /// .nest("/ui/", Ui::new())
    pub(super) fn new() -> Router<AppState> {
        for filename in <ASSETS as RustEmbed>::iter() {
            debug!("Assets include -> {filename}");
        }

        Router::new()
            .route("/", get(index_html))
            .route("/*path", get(file_serve))
            .fallback(not_found)
    }
}

#[cfg(not(debug_assertions))]
async fn index_html() -> Response {
    file_index_html().await
}

#[cfg(debug_assertions)]
async fn index_html(State(AppState { proxy_mode, .. }): State<AppState>) -> Response {
    match proxy_mode {
        UiProxyMode::Embed => {
            //
            file_index_html().await
        }
        UiProxyMode::Proxy => {
            //
            proxy_file_serve("").await
        }
    }
}

#[cfg(not(debug_assertions))]
async fn file_serve(Path(path): Path<String>) -> Response {
    embed_file_serve(&path).await
}

#[cfg(debug_assertions)]
async fn file_serve(
    Path(path): Path<String>,
    State(AppState { proxy_mode, .. }): State<AppState>,
    ConnectInfo(MyConnectInfo { remote, .. }): ConnectInfo<MyConnectInfo>,
) -> Response {
    match proxy_mode {
        UiProxyMode::Embed => {
            //
            embed_file_serve(&path).await
        }
        UiProxyMode::Proxy => {
            //
            proxy_file_serve(&path).await
        }
    }
}

#[cfg(debug_assertions)]
async fn proxy_file_serve(path: &str) -> Response {
    use hyper::StatusCode;

    let url = format!("{VITE_URL}{path}");
    info!(requesttoUrl = ?url);
    let client = reqwest::Client::builder().http1_only().build().unwrap();

    let reqwest_response = match client.get(url).send().await {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(%err, "request failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let mut response_builder = Response::builder().status(reqwest_response.status().as_u16());
    let mut headers = HeaderMap::with_capacity(reqwest_response.headers().len());
    headers.extend(reqwest_response.headers().into_iter().map(|(name, value)| {
        let name = HeaderName::from_bytes(name.as_ref()).unwrap();
        let value = HeaderValue::from_bytes(value.as_ref()).unwrap();
        (name, value)
    }));
    *response_builder.headers_mut().unwrap() = headers;

    response_builder
        .body(Body::from_stream(reqwest_response.bytes_stream()))
        // Same goes for this unwrap
        .unwrap()
}

async fn embed_file_serve(path: &str) -> Response {
    // /ui/a    -> /a
    // /ui/a/b  -> /a/b
    // /ui/a/b/ -> /a/b/
    let path = path.trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return file_index_html().await;
    }

    match ASSETS::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(hyper::header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }

            file_index_html().await
        }
    }
}

async fn file_index_html() -> Response {
    match ASSETS::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404 NotFound").into_response()
}

/*
#[cfg(not(debug_assertions))]
async fn ui_handler(req: Request) -> impl IntoResponse {
    Self::ui_static_handler(req).await
}

#[cfg(debug_assertions)]
async fn ui_handler(
    State(AppState {
        channel_manager,
        config_path,
        config,
        session_id,
        proxy_mode,
        manager_sender,
    }): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    use super::AppState;

    debug!("proxy_mode: {proxy_mode:?} -> {}", req.uri().path());
    match proxy_mode {
        UiProxyMode::Embed => {
            //
            Self::ui_static_handler(req).await.into_response()
        }
        UiProxyMode::Proxy => {
            //
            Self::ui_proxy_handler(req).await.into_response()
        }
    }
}

#[inline]
async fn ui_static_handler(req: Request) -> impl IntoResponse {
    let mut path = req.uri().path().trim_start_matches('/').to_string();
    if path == "ui" {
        // "ui" -> ""
        path = path.replace("ui", "");
    } else if path.starts_with("ui/") {
        // "ui/" -> ""
        // "ui/index.html" -> "index.html"
        path = path.replace("ui/", "");
    }

    StaticFile(path)
}

#[inline]
async fn ui_proxy_handler(req: Request) -> impl IntoResponse {
    let uri = req.uri();
    let url = format!("http://127.0.0.1:5173{uri}");

    let reqwest_response = match reqwest::Client::new().get(url).send().await {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(%err, "request failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let mut response_builder = Response::builder().status(reqwest_response.status());
    *response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();

    response_builder
        .body(Body::from_stream(reqwest_response.bytes_stream()))
        // Same goes for this unwrap
        .unwrap()
}
 */

// include!("static_file.inc");

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[derive(RustEmbed)]
#[folder = "client/dist/"]
#[include = "*.{html,js,css,svg}"]
struct ASSETS;

pub struct StaticFile<T>(pub T);
impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path: String = self.0.into();
        let path = match path.as_str() {
            "" => "index.html",
            p => &path,
        };
        trace!(?path);

        if let Some(content) = ASSETS::get(path) {
            trace!("StaticFile(path: {path}) discovered");
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            return ([(hyper::header::CONTENT_TYPE, mime.as_ref())], content.data).into_response();
        }
        let dir_filename = path.split("/").last().unwrap();
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    }
}
