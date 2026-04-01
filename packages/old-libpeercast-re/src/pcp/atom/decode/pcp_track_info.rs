use serde::Serialize;
use tracing::{error, warn};

use crate::{
    error::AtomParseError,
    pcp::{atom::decode::decode_string, decode::macros::merge_ref, Atom, Id4, TrackInfo},
};

use super::macros::getter;

#[derive(Debug, Clone, Default)]
pub struct PcpTrackInfo {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub url: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>, // only PeerCastStation?
}

impl PcpTrackInfo {
    #[tracing::instrument]
    pub fn parse(atom: &Atom) -> Result<PcpTrackInfo, AtomParseError> {
        if !(atom.id() == Id4::PCP_CHAN_TRACK && atom.is_parent()) {
            return Err(AtomParseError::ValueError);
        }

        let mut i = PcpTrackInfo::default();
        for a in atom.as_parent().childs() {
            if a.is_child() {
                let a = a.as_child();
                match a.id() {
                    Id4::PCP_CHAN_TRACK_TITLE => i.title = Some(decode_string(a)?),
                    Id4::PCP_CHAN_TRACK_CREATOR => i.creator = Some(decode_string(a)?),
                    Id4::PCP_CHAN_TRACK_URL => i.url = Some(decode_string(a)?),
                    Id4::PCP_CHAN_TRACK_ALBUM => i.album = Some(decode_string(a)?),
                    Id4::PCP_CHAN_TRACK_GENRE => i.genre = Some(decode_string(a)?),
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            } else {
                warn!("unkown atom arrived :{:?}", &a)
            }
        }

        Ok(i)
    }

    pub fn merge_ref(&mut self, other: &Self) -> bool {
        merge_ref!(self, other, [title, creator, url, album, genre])
    }

    getter!(&self, title);
    getter!(&self, creator);
    getter!(&self, url);
    getter!(&self, album);
    getter!(&self, genre);
}
