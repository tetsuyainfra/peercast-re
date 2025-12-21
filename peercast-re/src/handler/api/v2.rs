/// src/handler/api/v1.rs
/// API v1 handler
///
use std::sync::Arc;
use axum::{extract::State, routing};

use crate::peercast::Store;
use utoipa::OpenApi;

pub fn router(store: &Arc<Store>) -> axum::Router {
    axum::Router::new()
        .route("/", routing::get(root_handler))
        .with_state(store.clone())
}

#[derive(OpenApi)]
#[openapi(
    paths(root_handler),
    tags(
        (name = "v2", description = "API v2")
    )
)]
pub(super) struct ApiV2;

#[utoipa::path(
    get,
    path = "", // axumのルートパス指定と異なることに注意
    responses(
        (status = 200, description = "API v2 root preserved.", body = String)
    )
)]
pub(super) async fn root_handler() -> impl axum::response::IntoResponse {
    "API v2 root preserved."
}
