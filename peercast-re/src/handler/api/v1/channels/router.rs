use std::str::FromStr;

use axum::extract::{Path, State};
use axum::{Json, routing};
use http::StatusCode;
use libpeercast_re::pcp::GnuId;
use serde_json::json;
use utoipa::OpenApi;

use super::{JsonChannel, JsonTrack};
use crate::AppState;
use crate::prelude::*; // for instrument

#[derive(OpenApi)]
#[openapi(
    //
    paths(list_channels, create_channel, show_channel),
)]
pub struct ApiChannels;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(list_channels).post(create_channel))
        .route("/{id}", routing::get(show_channel))
}

///////////////////////////////////////////////////////////////////////////////
// Channels Handlers
//

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "get channels list")
    )
)]
#[instrument(skip(store))]
async fn list_channels(State(store): State<AppState>) -> impl axum::response::IntoResponse {
    let channels = store.repository.get_channels();

    let json_channels: Vec<JsonChannel> = channels.iter().map(|ch| JsonChannel::from(ch)).collect();
    axum::Json(json_channels)
}

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 200, description = "create channel")
    )
)]
#[instrument(skip(store))]
async fn create_channel(State(store): State<AppState>) -> impl axum::response::IntoResponse {
    let channels = store.repository.get_channels();
    "create channel"
    // Repository()
    //     .create_or_get(id, channel_info, track_info, config)
}

#[utoipa::path(
    get,
    path = "/{id}",
    params(
        ("id" = String, Path, description = "Channel ID", example = "00000000000000000123456789ABCDEF"),
    ),
    responses(
        (status = 200, description = "show channel")
    )
)]
#[instrument(skip(store))]
async fn show_channel(State(store): State<AppState>, Path(path): Path<String>) -> impl axum::response::IntoResponse {
    let channel_id = match GnuId::from_str(&path) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid channel id"}))),
    };

    match store.repository.get(&channel_id) {
        Some(ch) => {
            let json_channel: JsonChannel = JsonChannel::from(&ch);
            return (StatusCode::OK, Json(json!(json_channel)));
        }
        None => return (StatusCode::NOT_FOUND, Json(json!({"error": "channel not found"}))),
    };
}

#[utoipa::path(
    delete,
    path = "",
    responses(
        (status = 200, description = "delete channel")
    )
)]
#[instrument(skip(_store))]
async fn delete_channel(State(_store): State<AppState>) -> impl axum::response::IntoResponse {
    "delete channel"
}
