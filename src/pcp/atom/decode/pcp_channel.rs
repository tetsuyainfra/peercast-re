use tracing::{error, warn};

use crate::{
    error::AtomParseError,
    pcp::{atom::decode::decode_string, Atom, GnuId, Id4, TrackInfo},
};

use super::{decode_gnuid, PcpChannelInfo, PcpTrackInfo};

#[derive(Debug, Clone, Default)]
pub struct PcpChannel {
    pub channel_id: Option<GnuId>,
    pub broadcast_id: Option<GnuId>,
    pub channel_info: Option<PcpChannelInfo>,
    pub track_info: Option<PcpTrackInfo>,
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
