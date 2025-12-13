use std::sync::Arc;

use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::app::Store;

const RE_TAG: &str = "peercast-re";

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = RE_TAG, description = "PeerCast Re: API")
    )
)]
pub struct ApiDoc;

#[derive(Debug, Serialize)]
pub struct Channel {}

pub fn build_api(store: Arc<Store>) -> (axum::Router, utoipa::openapi::OpenApi) {
    let open_api_router = OpenApiRouter::new()
        .routes(routes!(list_channels, create_channel))
        // .routes(routes!(ip_check))
        // .routes(routes!(port_check))
        .with_state(store);

    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", open_api_router)
        .split_for_parts()
}

pub fn build_router(store: Arc<Store>) -> axum::Router {
    let (router, _api) = build_api(store);
    router
}

#[utoipa::path(get, path = "/channels")]
async fn list_channels(State(_store): State<Arc<Store>>) -> Json<Vec<Channel>> {
    let channels = vec![Channel {}];
    Json(channels)
}

#[utoipa::path(post, path = "/channels/create")]
async fn create_channel() -> Json<Channel> {
    Json(Channel {})
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn test_list_channels() {
        let store = Arc::new(Store {
            config: Default::default(),
            config_path: PathBuf::new(),
        });
        let response = list_channels(State(store)).await;
        let channels = response.0;
        assert_eq!(channels.len(), 1);
    }

    #[tokio::test]
    async fn test_create_channel() {
        let response = create_channel().await;
        let channel = response.0;
        // Add assertions as needed for channel fields
        unimplemented!();
    }
}
