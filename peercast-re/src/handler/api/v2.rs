use axum::routing;
/// src/handler/api/v1.rs
/// API v1 handler
///
use utoipa::OpenApi;

use crate::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(root_handler),
    tags(
        (name = "v2", description = "API v2")
    )
)]
pub(super) struct ApiV2;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(root_handler))
}

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
