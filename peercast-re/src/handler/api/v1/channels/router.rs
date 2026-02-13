use std::str::FromStr;

use axum::extract::{Path, State};
use axum::{Json, routing};
use http::StatusCode;
use libpeercast_re::pcp::GnuId;
use serde_json::json;
use utoipa::OpenApi;

use super::JsonChannel;
use crate::AppState;
use crate::prelude::*; // for instrument

#[derive(OpenApi)]
#[openapi(
    //
    paths(list_channels, create_channel, show_channel, update_channel, delete_channel),
)]
pub struct ApiChannels;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(list_channels).post(create_channel))
        .route("/{id}", routing::get(show_channel).post(update_channel).delete(delete_channel))
}

///////////////////////////////////////////////////////////////////////////////
// Channels Handlers
//

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "get channels list", body = Vec<JsonChannel>)
    )
)]
#[instrument(skip(store))]
async fn list_channels(State(store): State<AppState>) -> impl axum::response::IntoResponse {
    let json_channels = store.repository.map_collect(|_id, ch| JsonChannel::from(ch));

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
    let _channels = store.repository.get_channels();
    // Repository()
    //     .create_or_get(id, channel_info, track_info, config)
    "create channel"
}

#[utoipa::path(
    get,
    path = "/{id}",
    params(
        ("id" = String, Path, description = "Channel ID", example = json!(crate::DUMMY_CHANNEL_ID)),
    ),
    responses(
        (status = 200, description = "show channel", body = JsonChannel)
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
    post,
    path = "/{id}",
    params(
        ("id" = String, Path, description = "Channel ID", example = json!(crate::DUMMY_CHANNEL_ID)),
    ),
    responses(
        (status = 200, description = "update channel")
    )
)]
#[instrument(skip(store))]
async fn update_channel(State(store): State<AppState>, Path(path): Path<String>) -> impl axum::response::IntoResponse {
    let channel_id = match GnuId::from_str(&path) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid channel id"}))),
    };

    // TODO: inplement update logic here
    if let Some(_ch) = store.repository.get(&channel_id) {
        // TODO: update channel fields
        (StatusCode::OK, Json(json!({"message": "channel updated"})))
    } else {
        (StatusCode::NOT_FOUND, Json(json!({"error": "channel not found"})))
    }
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
    // TODO: implement delete logic here
    "delete channel"
}
