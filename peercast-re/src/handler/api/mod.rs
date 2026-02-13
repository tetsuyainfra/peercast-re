use axum::routing;

// src/handler/api/mod.rs
// API handler
//
use crate::AppState;

pub mod v1;
pub mod v2;

pub fn build_router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(versions))
        .nest("/v1", v1::router())
        .nest("/v2", v2::router())
}

async fn versions() -> impl axum::response::IntoResponse {
    let versions = vec!["v1"];
    axum::Json(versions)
}
