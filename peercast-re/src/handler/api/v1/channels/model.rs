use axum::{extract::State, routing};
use chrono::{DateTime, Utc};
use libpeercast_re::pcp::{ChannelInfo, GnuId, TrackInfo};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::{
    AppState,
    channel::ReChannel,
    prelude::*,
    repository::{Channel, ChannelType},
};

///////////////////////////////////////////////////////////////////////////////
// Response structs
//
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonChannel {
    pub id: String,
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
    pub created_at: String,
    pub track: JsonTrack,

    #[serde(rename = "type")]
    pub typee: String,

    pub channel_type: JsonChannelType,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum JsonChannelType {
    Root,
    Tracker,
    Relay,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JsonTrack {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String,
}

impl From<&ReChannel> for JsonChannel {
    fn from(ch: &ReChannel) -> Self {
        let ChannelInfo {
            typ,
            name,
            genre,
            desc,
            comment,
            url,
            stream_type,
            stream_ext,
            bitrate,
        } = ch.channel_info();

        JsonChannel {
            id: ch.id().to_string(),
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
            created_at: ch.created_at().to_rfc3339(),
            track: ch.track_info().into(),
            channel_type: ch.channel_type().into(),
        }
    }
}

impl From<ChannelType> for JsonChannelType {
    fn from(ct: ChannelType) -> Self {
        match ct {
            ChannelType::Root => JsonChannelType::Root,
            ChannelType::Tracker => JsonChannelType::Tracker,
            ChannelType::Relay => JsonChannelType::Relay,
        }
    }
}

impl From<TrackInfo> for JsonTrack {
    fn from(t: TrackInfo) -> Self {
        JsonTrack {
            title: t.title,
            creator: t.creator,
            url: t.url,
            album: t.album,
            genre: t.genre,
        }
    }
}
