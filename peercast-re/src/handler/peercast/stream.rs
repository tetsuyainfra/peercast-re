use std::{net::SocketAddr, str::FromStr};

use axum::{
    body::Body,
    extract::{Path, State},
    response::{self},
};
use axum_extra::extract::Host;
use http::StatusCode;
use libpeercast_re::{ConnectionNo, pcp::GnuId};

use crate::{AppState, channel::ReChannel, prelude::*, repository::Channel};

////////////////////////////////////////////////////////////////////
/// /stream/{channel_id_with_extention} handler
pub(super) async fn stream_handler(
    Host(_host): Host,
    Path(channel_id_with_ext): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, StatusCode> {
    info!("Stream requested for channel_id_extensions {}", channel_id_with_ext);
    let parts: Vec<&str> = channel_id_with_ext.split('.').collect();
    if parts.is_empty() {
        error!("Channel ID with extension is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    // 拡張子を無視してチャンネルIDを取得
    let channel_id_str = parts[0];

    // Channel IDをパース
    let channel_id = GnuId::from_str(channel_id_str).map_err(|_| {
        error!("Invalid channel ID format: {}", channel_id_str);
        StatusCode::BAD_REQUEST
    })?;

    let ch = state.repository.get(&channel_id);
    let ch = ch.ok_or_else(|| StatusCode::NOT_FOUND)?;

    // TODO: 拡張子を無視してFLVストリームを返しているのでどうするか要検討
    // 現状は常にFLVストリームを返す。ダメならこのスレッドが死ぬだけなので問題ないはず
    stream_flv(ch).await
}

async fn stream_flv(channel: ReChannel) -> Result<axum::response::Response, StatusCode> {
    // チャンネルのストリームを取得
    let stream = channel.channel_stream(ConnectionNo::new()).await.map_err(|e| {
        error!("Failed to get channel stream for channel {}, {:?}", channel.id(), e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // ストリームをHTTPのボディにラップ
    let body = Body::from_stream(stream);

    let response = response::Response::builder()
        .status(StatusCode::OK)
        // TODO: チャンネルの種類に応じてContent-Typeを変える
        .header(http::header::CONTENT_TYPE, "video/x-flv")
        .body(body)
        .unwrap();

    debug!("Stream response create success for channel {}", channel.id());
    Ok(response)
}
