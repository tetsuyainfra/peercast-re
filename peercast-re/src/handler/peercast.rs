use std::{net::SocketAddr, str::FromStr};

use axum::{
    extract::{Path, Query},
    routing,
};
use http::{StatusCode, header};
use libpeercast_re::pcp::GnuId;
use serde::Deserialize;

use crate::{AppState, channel::ReConfig, peercast::Repository};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/pls/{channel_id}", routing::get(pls_handler))
        .route("/stream/{channel_id}", routing::get(stream_handler))
    // TODO: /admin?cmd=viewxmlの実装
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

////////////////////////////////////////////////////////////////////
// Handlers for PeerCast requests
//
async fn pls_handler(
    Path(channel_id): Path<GnuId>,
    Query(params): Query<PlsParams>,
) -> impl axum::response::IntoResponse {
    let ch = Repository().create_or_get(
        channel_id,
        None,
        None,
        Some(ReConfig {
            tracker_ip: params.tip,
        }),
    );

    (
        //
        StatusCode::OK,
        [(http::header::CONTENT_TYPE, "audio/x-mpegurl")],
        create_m3u_playlist(channel_id),
    )
}

async fn stream_handler() -> &'static str {
    // Placeholder implementation for PeerCast stream handling
    "PeerCast Stream Handler"
}

////////////////////////////////////////////////////////////////////
/// for misc functions
//
fn create_m3u_playlist(channel_id: GnuId) -> String {
    // Create M3U playlist content like below:
    // #EXTM3U
    // #EXTINF:-1,
    // http://localhost:61744/stream/9E5AF27AC8F66604C50C415C1217933A.flv
    vec![
        //
        "#EXTM3U",
        "#EXTINF:-1,",
        format!("http://localhost:61744/stream/{}.flv", channel_id).as_str(),
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
        let channel_id = GnuId::from_str("9E5AF27AC8F66604C50C415C1217933A").unwrap();
        let playlist = create_m3u_playlist(channel_id);
        let expected = "#EXTM3U\n#EXTINF:-1,\nhttp://localhost:61744/stream/9E5AF27AC8F66604C50C415C1217933A.flv";
        assert_eq!(playlist, expected);
    }
}
