use std::{net::SocketAddr, str::FromStr, time::SystemTime};

use axum::{
    extract::{self, Path, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use axum_core::response::IntoResponse;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::debug;

use crate::{
    pcp::{Channel, ChannelInfo, ChannelType, GnuId, RelayTaskConfig, TaskStatus, TrackInfo},
    ConnectionId,
};
use peercast_re_api::models::{
    channel_info,
    channel_type::{self, Typ as ChannelTypeEnum},
    ChannelInfo as RespChannelInfo, ChannelStatus, ChannelTrack as RespChannelTrack,
    ChannelType as RespChannelType, ReqCreateChannel, ReqCreateRelayChannel, ReqPatchChannel,
    RespChannel,
};

use super::AppState;

pub(super) struct ChannelsSvc;

impl ChannelsSvc {
    pub(super) fn new() -> Router<AppState> {
        Router::new()
            .route("/", get(Self::list).post(Self::create))
            .route("/relay", post(Self::create_relay))
            .route("/:id", patch(Self::patch).delete(Self::delete))
    }

    async fn list(
        State(AppState {
            channel_manager, ..
        }): State<AppState>,
    ) -> impl IntoResponse {
        let channels: Vec<RespChannel> = channel_manager
            .map_collect(|(id, ch)| ch.clone())
            .iter()
            .map(|ch| RespChannel::from(ch))
            .collect();

        (StatusCode::OK, Json(channels))
    }

    async fn create(
        State(AppState {
            channel_manager, ..
        }): State<AppState>,
        extract::Json(info): extract::Json<ReqCreateChannel>,
    ) -> impl IntoResponse {
        debug!("json ch_info: {info:#?}");
        let ch_type = ChannelType::Broadcast;
        let channel_info = ChannelInfo::from(info);

        let Some(ch) = channel_manager.create(
            GnuId::new(),
            ch_type,
            channel_info.into(),
            TrackInfo::default().into(),
        ) else {
            return (StatusCode::BAD_REQUEST).into_response();
        };

        (StatusCode::CREATED, Json(RespChannel::from(&ch))).into_response()
    }

    async fn create_relay(
        State(AppState {
            channel_manager, ..
        }): State<AppState>,
        extract::Json(info): extract::Json<ReqCreateRelayChannel>,
    ) -> impl IntoResponse {
        debug!("json ch_info: {info:#?}");
        let Ok(channel_id) = GnuId::from_str(&info.id) else {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        };
        let Ok(addr) = info.host.parse::<SocketAddr>() else {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        };

        let channel_type = ChannelType::Relay;
        let Some(ch) = channel_manager.create(channel_id, channel_type, None, None) else {
            return (StatusCode::BAD_REQUEST).into_response();
        };
        let session_id = GnuId::new();
        let r = ch.connect(
            ConnectionId::new(),
            crate::pcp::SourceTaskConfig::Relay(RelayTaskConfig {
                addr,
                self_addr: todo!(),
            }),
        );

        (StatusCode::CREATED, Json(RespChannel::from(&ch))).into_response()
    }

    async fn patch(
        Path(channel_id): Path<String>,
        State(AppState {
            channel_manager, ..
        }): State<AppState>,
        // extract::Json(req_ch): extract::Json<ReqPatchChannel>,
        extract::Json(req_ch): extract::Json<ReqPatchChannel>,
    ) -> impl IntoResponse {
        let Ok(channel_id) = GnuId::from_str(&channel_id) else {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        };
        let Some(channel) = channel_manager.get(&channel_id) else {
            return (StatusCode::NOT_FOUND).into_response();
        };
        match channel.channel_type() {
            ChannelType::Broadcast { .. } => {}
            _ => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        }

        // // 操作
        // if req_ch.info.is_some() {
        //     let info = req_ch.info.unwrap();
        //     if (info.url.is_some()) {}
        // }
        // if req_ch.status.is_some() {
        //     //
        // }

        (
            StatusCode::OK,
            Json(json!({
                "result": "ok",
                "update_channel": channel_id
            })),
        )
            .into_response()
    }

    async fn delete(
        Path(channel_id): Path<String>,
        State(AppState {
            channel_manager, ..
        }): State<AppState>,
    ) -> impl IntoResponse {
        let Ok(channel_id) = GnuId::from_str(&channel_id) else {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        };
        if !channel_manager.delete(&channel_id) {
            return (StatusCode::NOT_FOUND).into_response();
        };

        (
            StatusCode::OK,
            Json(json!({
                "result": "ok",
                "delete_channel": channel_id
            })),
        )
            .into_response()
    }
}

macro_rules! insert_some_value {
    ($from:ident, $to:ident, $name:ident) => {
        if $from.$name.is_some() {
            $to.$name = $from.$name.take().unwrap()
        }
    };
}

impl From<ReqCreateChannel> for ChannelInfo {
    fn from(mut value: ReqCreateChannel) -> Self {
        let mut info = ChannelInfo::new();
        info.name = value.name;

        insert_some_value!(value, info, genre);
        insert_some_value!(value, info, desc);
        insert_some_value!(value, info, comment);
        insert_some_value!(value, info, url);

        info
    }
}

impl From<&TrackInfo> for RespChannelTrack {
    fn from(value: &TrackInfo) -> Self {
        let TrackInfo {
            title,
            creator,
            url,
            album,
            genre,
        } = value.clone();
        Self {
            title: title,
            creator: creator,
            url: url,
            album: album,
            genre: Some(genre),
        }
    }
}

impl From<&ChannelInfo> for RespChannelInfo {
    fn from(value: &ChannelInfo) -> Self {
        use num::FromPrimitive;
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
        } = value.clone();
        Self {
            typ: typ,
            name: name,
            genre: genre,
            desc: desc,
            comment: comment,
            url: url,
            stream_type: stream_type,
            stream_ext: stream_ext,
            bitrate: i32::from_i32(value.bitrate).unwrap_or(i32::MAX),
        }
    }
}

impl From<&TaskStatus> for ChannelStatus {
    fn from(value: &TaskStatus) -> Self {
        match value {
            TaskStatus::Init => ChannelStatus::Init,
            TaskStatus::Searching { searched, all } => ChannelStatus::Searching,
            TaskStatus::Receiving => ChannelStatus::Receiving,
            TaskStatus::Idle => ChannelStatus::Idle,
            TaskStatus::Finish => ChannelStatus::Finish,
            TaskStatus::Error => ChannelStatus::Error,
        }
    }
}

impl From<&ChannelType> for RespChannelType {
    fn from(value: &ChannelType) -> Self {
        match value {
            ChannelType::Broadcast => RespChannelType::new(ChannelTypeEnum::Broadcast, "".into()),
            ChannelType::Relay => RespChannelType::new(ChannelTypeEnum::Relay, "".into()),
        }
    }
}

impl From<&Channel> for RespChannel {
    fn from(value: &Channel) -> Self {
        let info = value.info().unwrap_or_default();
        let track = value.track().unwrap_or_default();
        Self {
            id: value.id().to_string(),
            channel_type: Box::new(RespChannelType::from(&value.channel_type())),
            info: Box::new(RespChannelInfo::from(&info)),
            track: Box::new(RespChannelTrack::from(&track)),
            status: ChannelStatus::from(&value.status()),
            created_at: value.created_at().to_rfc3339(),
        }
    }
}
