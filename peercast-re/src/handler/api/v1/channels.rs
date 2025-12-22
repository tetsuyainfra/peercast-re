use std::net::SocketAddr;

use axum::{extract::State, routing};
use chrono::{DateTime, Utc};
use libpeercast_re::pcp::{ChannelInfo, GnuId, TrackInfo};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;

use crate::{AppState, channel::ReChannel, peercast::Repository, prelude::*};

#[derive(OpenApi)]
#[openapi(
    //
    paths(list_channels, create_channel),
)]
pub(super) struct ApiChannels;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        //
        .route("/", routing::get(list_channels))
        .route("/create", routing::post(create_channel))
    // .fallback(routing::get(list_channels))
}

///////////////////////////////////////////////////////////////////////////////
// Response structs
//
#[derive(Debug, Clone, Serialize)]
pub struct JsonChannel {
    pub id: GnuId,
    pub name: String,
    // pub tracker_addr: Option<SocketAddr>,
    pub contact_url: String,
    pub genre: String,
    pub raw_genre: String, // namespace, listener_hideable, PortLimitを含むgenre
    pub desc: String,
    pub comment: String,
    /// MIME
    pub stream_type: String,
    /// 拡張子
    pub stream_ext: String,
    pub bitrate: i32,
    // filetype: String,
    // status: String,
    pub number_of_listener: i32,
    pub number_of_relay: i32,
    pub created_at: DateTime<Utc>, // FIX: 外部のCDNなどとの兼ね合いで配信時間が00:00意外になる可能性あり
    pub track: JsonTrack,

    #[serde(rename = "type")]
    pub typee: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonTrack {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String,
}

impl From<&ReChannel> for JsonChannel {
    fn from(ch: &ReChannel) -> Self {
        let ChannelInfo { typ, name, genre, desc, comment, url, stream_type, stream_ext, bitrate } = ch.channel_info();

        JsonChannel {
            id: ch.id(),
            name,
            contact_url: url,
            genre: genre.clone(),
            raw_genre: genre,
            desc,
            comment,
            typee: typ,
            stream_type,
            stream_ext,
            bitrate,
            number_of_listener: ch.number_of_listener(),
            number_of_relay: ch.number_of_relay(),
            created_at: ch.created_at(),
            track: ch.track_info().into(),
        }
    }
}

impl From<TrackInfo> for JsonTrack {
    fn from(t: TrackInfo) -> Self {
        JsonTrack { title: t.title, creator: t.creator, url: t.url, album: t.album, genre: t.genre }
    }
}

///////////////////////////////////////////////////////////////////////////////
// Handlers
//

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "get channels list")
    )
)]
#[instrument(skip(_store))]
async fn list_channels(State(_store): State<AppState>) -> impl axum::response::IntoResponse {
    let channels = Repository().get_channels();

    let json_channels: Vec<JsonChannel> = channels.iter().map(|ch| JsonChannel::from(ch)).collect();
    axum::Json(json_channels)
}

#[utoipa::path(
    get,
    path = "/create",
    responses(
        (status = 200, description = "create channel")
    )
)]
#[instrument(skip(_store))]
async fn create_channel(State(_store): State<AppState>) -> impl axum::response::IntoResponse {
    "list channels"
    // Repository()
    //     .create_or_get(id, channel_info, track_info, config)
}
