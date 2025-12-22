use axum::{extract::State, routing};
use utoipa::OpenApi;

use crate::AppState;

pub mod channels;

////////////////////////////////////////////////////////////////////////////////
// API
//
#[derive(OpenApi)]
#[openapi(
    paths(list_users, get_config),
    nest(
        (path = "/channels", api = channels::ApiChannels),
    ),
    tags(
        (name = "v1", description = "API v1")
    )
)]
pub(super) struct ApiV1;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/users", routing::get(list_users))
        .route("/config", routing::get(get_config))
        .nest("/channels", channels::router())
}

////////////////////////////////////////////////////////////////////////////////
// Handlers
//
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
pub(super) async fn get_config(State(store): State<AppState>) -> impl axum::response::IntoResponse {
    let config_path = store.config_path.clone();

    #[derive(Debug, serde::Serialize)]
    struct Config {
        config_path: String,
    }

    axum::Json(Config {
        config_path: config_path.to_string_lossy().to_string(),
    })
}
