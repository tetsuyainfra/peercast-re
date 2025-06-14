use std::ops::Deref;

use bytes::Bytes;
use merge::Merge;
use serde::Serialize;
use tracing::warn;

use crate::{
    error::AtomParseError,
    pcp::{
        atom::decode::{self, decode_i32, decode_string},
        decode::macros::merge_ref,
        Atom, Channel, ChannelInfo, ChildAtom, Id4,
    },
};

use super::macros::getter;

#[derive(Debug, Clone, Default, Merge)]
pub struct PcpChannelInfo {
    // FLV, WMV, RAWなどのタイプ・・・
    pub typ: Option<String>,
    pub name: Option<String>,
    pub genre: Option<String>,
    pub desc: Option<String>,
    pub comment: Option<String>,
    pub url: Option<String>,
    // MIME識別子
    pub stream_type: Option<String>,
    // .で始まる拡張子
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

    pub fn merge_ref(&mut self, other: &Self) -> bool {
        merge_ref!(
            self,
            other,
            [
                typ,
                name,
                genre,
                desc,
                comment,
                url,
                stream_type,
                stream_ext,
                bitrate
            ]
        )
    }

    // --- Getter ---A
    getter!(&self, typ);
    getter!(&self, name);
    getter!(&self, genre);
    getter!(&self, desc);
    getter!(&self, comment);
    getter!(&self, url);
    getter!(&self, stream_type);
    getter!(&self, stream_ext);
    getter!(&self, bitrate, i32, 0);
}

/*
#[derive(Debug, Clone, Default, Merge)]
pub struct PcpChannelInfoWith {
    pub typ: Option<String>,
    pub name: Option<String>,
    // pub genre: Option<String>,
    // pub desc: Option<String>,
    // pub comment: Option<String>,
    // pub url: Option<String>,
    // //
    // pub stream_type: Option<String>,
    // // .で始まる拡張子
    // pub stream_ext: Option<String>,
    // pub bitrate: Option<i32>,
}

pub(crate) struct PcpChannelInfoMerger<'a> {
    pub atom: &'a mut Atom,
}

impl<'a> PcpChannelInfoMerger<'a> {
    pub fn update_with(&mut self, other: &mut PcpChannelInfoWith) {
        let PcpChannelInfoWith { typ, name } = other;
    }

    pub fn update_by(mut self, id: Id4) {
        match self
            .atom
            .as_parent_mut()
            .childs_mut()
            .find(|t| t.id() == id)
        {
            Some(_) => todo!(),
            None => todo!(),
        }
    }
}
*/
