use std::sync::Arc;

use axum::{Json, extract::State};
use libpeercast_re::http::Api;
use serde::Serialize;
use serde_json::json;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

const RE_TAG: &str = "peercast-re";

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = RE_TAG, description = "PeerCast Re: API")
    )
)]
pub struct ApiDoc;

#[derive(Debug, Clone)]
pub struct ReStore {}

#[derive(Debug, Serialize)]
pub struct Channel {}

pub fn router(store: Arc<ReStore>) -> (axum::Router, utoipa::openapi::OpenApi) {
    let open_api_router = OpenApiRouter::new()
        .routes(routes!(list_channels, create_channel))
        // .routes(routes!(ip_check))
        // .routes(routes!(port_check))
        .with_state(store);

    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", open_api_router)
        .split_for_parts()
}

#[utoipa::path(get, path = "/channels")]
async fn list_channels(State(store): State<Arc<ReStore>>) -> Json<Vec<Channel>> {
    let channels = vec![Channel {}];
    Json(channels)
}

#[utoipa::path(post, path = "/channels/create")]
async fn create_channel() -> Json<Channel> {
    Json(Channel {})
}
