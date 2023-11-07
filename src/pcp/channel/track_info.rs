use serde::Serialize;
use tracing::error;

use crate::pcp::{atom::decode::PcpTrackInfo, Atom, Id4};

/// Channel's track info
#[derive(Debug, Clone, Default, Serialize)]
pub struct TrackInfo {
    pub title: String,
    pub creator: String,
    pub url: String,
    pub album: String,
    pub genre: String, // only PeerCastStation?
}

impl TrackInfo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl From<&PcpTrackInfo> for TrackInfo {
    fn from(pcp_track_info: &PcpTrackInfo) -> Self {
        let p = pcp_track_info.clone();
        TrackInfo {
            title: p.title.unwrap_or_default(),
            creator: p.creator.unwrap_or_default(),
            url: p.url.unwrap_or_default(),
            album: p.album.unwrap_or_default(),
            genre: p.genre.unwrap_or_default(),
        }
    }
}
