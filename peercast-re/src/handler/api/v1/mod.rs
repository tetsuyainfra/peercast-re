use axum::routing;
use utoipa::OpenApi;

use crate::AppState;

pub mod channels;
pub mod config;
pub(self) mod result_error;

////////////////////////////////////////////////////////////////////////////////
// API
//
#[derive(OpenApi)]
#[openapi(
    paths(list_users),
    nest(
        (path = "/channels", api = channels::ApiChannels),
    ),
    tags(
        (name = "v1", description = "API v1")
    )
)]
pub struct ApiV1;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/users", routing::get(list_users))
        // .route("/config", routing::get(get_config))
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
pub async fn list_users() -> impl axum::response::IntoResponse {
    "list users"
}
