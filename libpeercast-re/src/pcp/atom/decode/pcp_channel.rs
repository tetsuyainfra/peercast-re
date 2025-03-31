use std::sync::Arc;

use bytes::BufMut;
use tracing::{error, warn};

use crate::{
    error::AtomParseError,
    pcp::{atom::decode::decode_string, Atom, ChildAtom, GnuId, Id4, ParentAtom, TrackInfo},
};

use super::{
    decode_gnuid,
    pcp_channel_info::{self},
    PcpChannelInfo, PcpTrackInfo,
};

#[derive(Debug, Clone)]
pub struct PcpChannel {
    atom: Arc<Atom>,
    pub channel_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    pub channel_info: Option<PcpChannelInfo>,
    pub track_info: Option<PcpTrackInfo>,
}

impl Default for PcpChannel {
    fn default() -> Self {
        Self {
            atom: Arc::new(Atom::Parent(ParentAtom::new(Id4::PCP_CHAN, vec![]))),
            channel_id: Default::default(),
            broadcast_id: Default::default(),
            channel_info: Default::default(),
            track_info: Default::default(),
        }
    }
}

impl PcpChannel {
    #[tracing::instrument]
    pub fn parse(atom: &Atom) -> Result<Self, AtomParseError> {
        if !(atom.id() == Id4::PCP_CHAN && atom.is_parent()) {
            return Err(AtomParseError::ValueError);
        }

        let mut c = PcpChannel::default();
        for a in atom.as_parent().childs() {
            if a.is_child() {
                let a = a.as_child();
                match a.id() {
                    Id4::PCP_CHAN_ID => c.channel_id = Some(decode_gnuid(a)?),
                    Id4::PCP_CHAN_BCID => c.broadcast_id = Some(decode_gnuid(a)?),
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            } else {
                match a.id() {
                    Id4::PCP_CHAN_INFO => {
                        c.channel_info = Some(PcpChannelInfo::parse(a)?);
                    }
                    Id4::PCP_CHAN_TRACK => {
                        c.track_info = Some(PcpTrackInfo::parse(a)?);
                    }
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            }
        }

        Ok(c)
    }
}

/*
#[derive(Debug)]
pub struct PcpChannelWith {
    pub channel_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    pub channel_info: Option<PcpChannelInfoWith>,
    // pub track_info: Option<PcpTrackInfoWith>,
}

pub(crate) struct PcpChannelMerger<'a> {
    atom: &'a mut Atom,
    channel_info: PcpChannelInfoWith,
    // track_info:  PcpTrackInfoWith,
}

impl<'a> PcpChannelMerger<'a> {
    pub fn update_with(&mut self, other: &mut PcpChannelWith) {
        for a in self.atom.as_parent_mut().childs_mut() {
            match a.id() {
                Id4::PCP_CHAN_ID => {}
                Id4::PCP_CHAN_BCID => {}
                Id4::PCP_CHAN_INFO => {
                    let mut merger = PcpChannelInfoMerger { atom: a };
                    // merger.update_with(&mut self.info);
                }
                Id4::PCP_CHAN_TRACK => {}
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod t {
    use super::PcpChannel;

    #[test]
    fn test() {
        let pcp = PcpChannel::default();
    }
}
 */
