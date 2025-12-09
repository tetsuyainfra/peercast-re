use axum::{Router, http::{ StatusCode, Uri}, response::{Html, IntoResponse, Response}};
use rust_embed::Embed;
use tracing::debug;

// pub fn router() -> axum::Router {
//     Router::new().route("/ui", routing::get(ui_root))
// }
// async fn  ui_root() -> String{
//     "/ui".into()
// }

// pub fn router() -> axum::Router {
//     // Router::new().route("/ui", routing::get(ui_root))
//     let    d = ServeDir::new("client/dist");
//     Router::new().nest_service("/ui", ServeDir::new("client/dist"))
// }

// use tower::ServiceExt;
// use tower_http::{
//     services::{ServeDir
//     }
// };
// pub fn router() -> axum::Router {
//     Router::new().nest_service("/ui",
//         get(|request: Request| async {
//             tracing::debug!("Serving UI static file: {}", request.uri().path());
//             // tracing::debug!("pwd: {}", std::env::current_dir().unwrap().display());
//             let service = ServeDir::new("peercast-re/client/dist");
//             let result = service.oneshot(request).await;
//             result
//         })
//     )
// }

#[derive(Embed)]
#[folder = "client/dist/"]
struct Assets;
static INDEX_HTML: &str = "index.html";

pub fn build_router() -> axum::Router {
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
            // let mime = mime_guess::from_path(path).first_or_octet_stream();
            // ([(axum::http::header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            Html(content.data).into_response()
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
