use serde::Serialize;
use tracing::warn;

use crate::{
    error::AtomParseError,
    pcp::{
        atom::decode::{self, decode_i32, decode_string},
        Atom, Channel, ChannelInfo, Id4,
    },
};

#[derive(Debug, Clone, Default)]
pub struct PcpChannelInfo {
    pub typ: Option<String>,
    pub name: Option<String>,
    pub genre: Option<String>,
    pub desc: Option<String>,
    pub comment: Option<String>,
    pub url: Option<String>,
    pub stream_type: Option<String>,
    pub stream_ext: Option<String>,
    pub bitrate: Option<i32>,
}

impl PcpChannelInfo {
    #[tracing::instrument]
    pub fn parse(atom: &Atom) -> Result<PcpChannelInfo, AtomParseError> {
        if !(atom.id() == Id4::PCP_CHAN_INFO && atom.is_parent()) {
            return Err(AtomParseError::ValueError);
        }

        let mut i = PcpChannelInfo::default();
        for a in atom.as_parent().childs() {
            if a.is_child() {
                let a = a.as_child();
                match a.id() {
                    Id4::PCP_CHAN_INFO_TYPE => i.typ = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_NAME => i.name = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_GENRE => i.genre = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_DESC => i.desc = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_COMMENT => i.comment = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_URL => i.url = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_STREAMTYPE => i.stream_type = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_STREAMEXT => i.stream_ext = Some(decode_string(a)?),
                    Id4::PCP_CHAN_INFO_BITRATE => i.bitrate = Some(decode_i32(a)?),
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
}