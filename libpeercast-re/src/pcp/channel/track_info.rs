use serde::Serialize;
use tracing::error;

use crate::pcp::{atom::decode::PcpTrackInfo, Atom, Id4};

use super::merge_field;

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

    pub fn merge_pcp(&mut self, new_val: PcpTrackInfo) {
        merge_field!(self, new_val, title);
        merge_field!(self, new_val, creator);
        merge_field!(self, new_val, url);
        merge_field!(self, new_val, album);
        merge_field!(self, new_val, genre);
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


#[cfg(test)]
mod t {
    use crate::pcp::{decode::PcpTrackInfo, TrackInfo};


    #[test]
    fn test_merge(){
        let mut ci = TrackInfo::new();
        let mut info = PcpTrackInfo::default();
        info.title = Some("title".into());
        let mut i = info.clone();

        ci.merge_pcp(i);

        assert_eq!(ci.title, info.title.unwrap());
    }

}