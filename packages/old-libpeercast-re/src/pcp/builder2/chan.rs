use bytes::Buf;
use tracing::warn;

use crate::pcp::{
    atom2::{
        decode2::{decode_gnuid, decode_i32, decode_vecu8},
        Atom2Kind, AtomView, ChildView, ParentView,
    },
    builder2::InfoParseError,
    Atom2, GnuId, Id4,
};
////////////////////////////////////////////////////////////////////////////////
//  ChanInfo
//
#[derive(Debug, Default)]
pub struct ChanInfo {
    pub channel_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    pub channel_info: Option<ChannelInfo>,
    pub track_info: Option<TrackInfo>,
}
impl TryFrom<&ParentView<'_>> for ChanInfo {
    type Error = InfoParseError;

    fn try_from(parent: &ParentView<'_>) -> Result<Self, Self::Error> {
        if (parent.id() != Id4::PCP_CHAN) {
            return Err(InfoParseError::TargetNotFound);
        }

        let mut ci = ChanInfo::default();
        for a in parent.children() {
            match a {
                Atom2Kind::Child(cv) => match cv.id() {
                    Id4::PCP_CHAN_ID => ci.channel_id = decode_gnuid(&cv).ok(),
                    Id4::PCP_CHAN_BCID => ci.broadcast_id = decode_gnuid(&cv).ok(),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &cv);
                    }
                },
                Atom2Kind::Parent(pv) => match pv.id() {
                    Id4::PCP_CHAN_INFO => ci.channel_info = ChannelInfo::try_from(&pv).ok(),
                    Id4::PCP_CHAN_TRACK => ci.track_info = TrackInfo::try_from(&pv).ok(),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &pv);
                    }
                },
            }
        }

        Ok(ci)
    }
}

impl TryFrom<&Atom2> for ChanInfo {
    type Error = InfoParseError;

    fn try_from(atom: &Atom2) -> Result<Self, Self::Error> {
        if (atom.id() != Id4::PCP_CHAN) {
            return Err(InfoParseError::TargetNotFound);
        }

        let Atom2Kind::Parent(pv) = atom.view() else {
            return Err(InfoParseError::TargetNotFound);
        };

        ChanInfo::try_from(&pv)
    }
}

////////////////////////////////////////////////////////////////////////////////
//  ChanChannelInfo
//

#[derive(Debug, Default)]
pub struct ChannelInfo {
    // FLV, WMV, RAWなどのタイプ・・・
    pub typee: Option<Vec<u8>>,
    pub name: Option<Vec<u8>>,
    pub genre: Option<Vec<u8>>,
    pub desc: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,
    pub url: Option<Vec<u8>>,
    // MIME識別子
    pub stream_type: Option<Vec<u8>>,
    // .で始まる拡張子
    pub stream_ext: Option<Vec<u8>>,
    pub bitrate: Option<i32>,
}

impl TryFrom<&ParentView<'_>> for ChannelInfo {
    type Error = InfoParseError;

    fn try_from(view: &ParentView<'_>) -> Result<Self, Self::Error> {
        if view.id() != Id4::PCP_CHAN_INFO {
            return Err(InfoParseError::TargetNotFound);
        }

        let mut ci = ChannelInfo::default();
        for a in view.children() {
            match a {
                Atom2Kind::Child(cv) => match cv.id() {
                    Id4::PCP_CHAN_INFO_NAME => ci.name = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_TYPE => ci.typee = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_GENRE => ci.genre = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_DESC => ci.desc = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_COMMENT => ci.comment = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_URL => ci.url = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_STREAMTYPE => ci.stream_type = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_STREAMEXT => ci.stream_ext = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_INFO_BITRATE => ci.bitrate = decode_i32(&cv).ok(),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &cv);
                    }
                },
                Atom2Kind::Parent(pv) => {
                    warn!("unknown atom arrived :{:?}", &pv);
                }
            }
        }

        Ok(ci)
    }
}

////////////////////////////////////////////////////////////////////////////////
//  ChanTrackInfo
//

#[derive(Debug, Default)]
pub struct TrackInfo {
    pub title: Option<Vec<u8>>,
    pub creator: Option<Vec<u8>>,
    pub url: Option<Vec<u8>>,
    pub album: Option<Vec<u8>>,
    pub genre: Option<Vec<u8>>, // only PeerCastStation?
}
impl TryFrom<&ParentView<'_>> for TrackInfo {
    type Error = InfoParseError;

    fn try_from(value: &ParentView<'_>) -> Result<Self, Self::Error> {
        if value.id() != Id4::PCP_CHAN_TRACK {
            return Err(InfoParseError::TargetNotFound);
        }

        let mut ti = TrackInfo::default();
        for a in value.children() {
            match a {
                Atom2Kind::Child(cv) => match cv.id() {
                    Id4::PCP_CHAN_TRACK_TITLE => ti.title = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_TRACK_CREATOR => ti.creator = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_TRACK_URL => ti.url = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_TRACK_ALBUM => ti.album = decode_vecu8(&cv).ok(),
                    Id4::PCP_CHAN_TRACK_GENRE => ti.genre = decode_vecu8(&cv).ok(),
                    _ => {
                        warn!("unknown atom arrived :{:?}", &cv);
                    }
                },
                Atom2Kind::Parent(pv) => {
                    warn!("unknown atom arrived :{:?}", &pv);
                }
            }
        }

        Ok(ti)
    }
}
