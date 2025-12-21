use std::sync::Arc;

use axum::{extract::State, routing};
/// src/handler/api/v1.rs
/// API v1 handler
///
use utoipa_axum::{router::OpenApiRouter, routes};

////////////////////////////////////////////////////////////////////////////////
// API
//
use crate::peercast::Store;
use utoipa::OpenApi;

pub fn router(store: &Arc<Store>) -> axum::Router {
    axum::Router::new()
        .route("/users", routing::get(list_users))
        .route("/config", routing::get(get_config))
        .with_state(store.clone())
}

#[derive(OpenApi)]
#[openapi(
    paths(list_users, get_config),
    tags(
        (name = "v1", description = "API v1")
    )
)]
pub(super) struct ApiV1;

#[utoipa::path(
    get,
    path = "/users",
    responses(
        (status = 200, description = "list users")
    )
)]
pub(super) async fn list_users() -> impl axum::response::IntoResponse {
    "list users"
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "get config")
    )
)]
pub(super) async fn get_config(
    State(store): State<Arc<Store>>,
) -> impl axum::response::IntoResponse {
    let config_path = store.config_path.clone();

    #[derive(Debug, serde::Serialize)]
    struct Config {
        config_path: String,
    }

    axum::Json(Config {
        config_path: config_path.to_string_lossy().to_string(),
    })
}
