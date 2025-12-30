use std::{net::SocketAddr, str::FromStr};

use axum::{
    body::Body,
    extract::{Path, Query, Request},
    response::{self, IntoResponse},
    routing,
};
use axum_extra::extract::Host;
use http::StatusCode;
use libpeercast_re::pcp::GnuId;
use serde::Deserialize;

use crate::{AppState, channel::ReConfig, prelude::*};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
    // .route("/pls/{channel_id}", routing::get(pls_handler))
    // .route("/stream/{channel_id}", routing::get(stream_handler))
    // TODO: /admin?cmd=viewxmlの実装
    // .route("/admin", routing::get(admin_handler))
    // ほかにもありそう
}
////////////////////////////////////////////////////////////////////
// Parameters for PeerCast requests
//
#[derive(Debug, Deserialize)]
struct PlsParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    tip: Option<SocketAddr>,
}

#[derive(Debug, Deserialize)]
struct AdminParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    cmd: Option<String>,
}

/// Serde deserialization decorator to map empty Strings to None,
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(serde::de::Error::custom).map(Some),
    }
}

/*
////////////////////////////////////////////////////////////////////
// Handlers for PeerCast requests
//
async fn pls_handler(
    Host(host): Host,
    Path(channel_id): Path<GnuId>,
    Query(params): Query<PlsParams>,
) -> impl axum::response::IntoResponse {
    info!("Request Host: {}", host);
    // requestからhostをとりだす
    let ch = Repository()
        .create_or_get(
            channel_id,
            None,
            None,
            Some(ReConfig {
                tracker_ip: params.tip,
            }),
        )
        .await;

    (
        //
        StatusCode::OK,
        [(http::header::CONTENT_TYPE, "audio/x-mpegurl")],
        create_m3u_playlist(&host, ch.id()),
    )
}
async fn stream_handler(
    Host(_host): Host,
    Path(channel_id): Path<GnuId>,
) -> Result<axum::response::Response, StatusCode> {
    info!("Stream requested for channel {}", channel_id);
    let ch = Repository().get(&channel_id);
    let ch = ch.ok_or_else(|| StatusCode::NOT_FOUND)?;

    //
    let stream = ch.channel_stream().map_err(|e| {
        error!("Failed to get channel stream for channel {}, {:?}", channel_id, e);
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

    debug!("Stream response create success for channel {}", channel_id);
    Ok(response)
} */

/*
async fn admin_handler(Query(_params): Query<AdminParams>) -> impl axum::response::IntoResponse {
    // <?xml version="1.0" encoding="utf-8"?>
    // <peercast session="EE22BBAD11B042BCA968128BC8735F9A">
    //   <servent uptime="361103" />
    //   <bandwidth in="2128" out="0" />
    //   <connections total="1" relays="0" direct="1" />
    //   <channels_relayed total="1">
    //     <channel id="73E669CBA92E7CA31143940B8DBFABBD" name="局長ch" bitrate="2128" comment="そこら辺の画像からでも生成できる・・・これがディープフェイクか" desc="18x AIエロ動画" genre="sp@@@game" type="FLV" url="https://jbbs.shitaraba.net/bbs/read.cgi/radio/25607/1766498555/" uptime="365" age="365" skip="0" bcflags="0">
    //       <hits hosts="0" listeners="0" relays="0" firewalled="0" closest="0" furthest="0" newest="0" />
    //       <relay listeners="1" relays="0" hosts="0" status="RECEIVE">
    //         <buffer bytes="812995" duration="2.977745" />
    //       </relay>
    //       <track title="" album="" genre="" artist="" contact="" />
    //     </channel>
    //   </channels_relayed>
    //   <channels_found total="1">
    //     <channel id="73E669CBA92E7CA31143940B8DBFABBD" name="局長ch" bitrate="2128" comment="そこら辺の画像からでも生成できる・・・これがディープフェイクか" desc="18x AIエロ動画" genre="sp@@@game" type="FLV" url="https://jbbs.shitaraba.net/bbs/read.cgi/radio/25607/1766498555/" uptime="365" age="365" skip="0" bcflags="0">
    //       <hits hosts="0" listeners="0" relays="0" firewalled="0" closest="0" furthest="0" newest="0" />
    //       <relay listeners="1" relays="0" hosts="0" status="RECEIVE">
    //         <buffer bytes="812995" duration="2.977745" />
    //       </relay>
    //       <track title="" album="" genre="" artist="" contact="" />
    //     </channel>
    //   </channels_found>
    // </peercast>
    "".into_response()
} */

////////////////////////////////////////////////////////////////////
/// for misc functions
//
fn create_m3u_playlist(host: &String, channel_id: GnuId) -> String {
    // Create M3U playlist content like below:
    // #EXTM3U
    // #EXTINF:-1,
    // http://localhost:61744/stream/9E5AF27AC8F66604C50C415C1217933A.flv
    vec![
        //
        "#EXTM3U",
        "#EXTINF:-1,",
        format!("http://{}/stream/{}.flv", host, channel_id).as_str(),
        "",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string_as_none() {
        let deserializer = serde_json::from_str::<PlsParams>(r#"{"tip":""}"#).unwrap();
        assert_eq!(deserializer.tip, None);
    }

    #[test]
    fn test_create_m3u_playlist() {
        let host = "localhost:61744".to_string();
        let channel_id = GnuId::from_str("9E5AF27AC8F66604C50C415C1217933A").unwrap();
        let playlist = create_m3u_playlist(&host, channel_id);
        let expected = "#EXTM3U\n#EXTINF:-1,\nhttp://localhost:61744/stream/9E5AF27AC8F66604C50C415C1217933A.flv\n";
        assert_eq!(playlist, expected);
    }
}
