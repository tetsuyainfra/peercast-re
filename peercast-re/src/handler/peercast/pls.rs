use std::{net::SocketAddr, str::FromStr};

use axum::extract::{Path, Query, State};
use axum_extra::extract::Host;
use http::StatusCode;
use libpeercast_re::pcp::GnuId;
use serde::Deserialize;

use crate::AppState;
use crate::channel::ReConfig;
use crate::prelude::*;
use crate::repository::Channel;

////////////////////////////////////////////////////////////////////
// Parameters for PeerCast requests
//
#[derive(Debug, Deserialize)]
pub(super) struct PlsParams {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    tip: Option<SocketAddr>,
}

// #[derive(Debug, Deserialize)]
// struct AdminParams {
//     #[serde(default, deserialize_with = "empty_string_as_none")]
//     cmd: Option<String>,
// }

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
/// /pls/{channel_id} handler
pub(super) async fn pls_handler(
    Host(host): Host,
    Path(channel_id): Path<GnuId>,
    Query(params): Query<PlsParams>,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    info!("Request Host: {}", host);
    // requestからhostをとりだす
    let ch = state
        .repository
        .create_or_get(
            channel_id,
            None,
            None,
            state.rtmp_manager_sender.clone(),
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
