use bytes::Bytes;
use tracing::warn;

use crate::{
    error::AtomParseError,
    pcp::{
        atom::decode::{
            decode_bytes, decode_gnuid, decode_i16, decode_i32, decode_u16, decode_u32, decode_u8,
            PcpChannel,
        },
        Atom, GnuId, Id4,
    },
};

use super::PcpHost;

#[derive(Debug)]
pub struct BroadcastGroup(u8);
impl BroadcastGroup {
    pub const TO_ALL: BroadcastGroup = BroadcastGroup(0xFF);
    pub const TO_ROOT: BroadcastGroup = BroadcastGroup(0x01);
    pub const TO_TRACKERS: BroadcastGroup = BroadcastGroup(0x02);
    pub const TO_RELAYS: BroadcastGroup = BroadcastGroup(0x04);

    fn has(&self, other: &BroadcastGroup) -> bool {
        (self.0 & other.0) != 0
    }
    pub fn is_all(&self) -> bool {
        self.0 == Self::TO_ALL.0
    }
    pub fn has_root(&self) -> bool {
        self.has(&Self::TO_ROOT)
    }
    pub fn has_trackers(&self) -> bool {
        self.has(&Self::TO_TRACKERS)
    }
    pub fn has_relays(&self) -> bool {
        self.has(&Self::TO_RELAYS)
    }
}

impl From<u8> for BroadcastGroup {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default)]
pub struct PcpBroadcast {
    pub ttl: Option<u8>,
    pub hops: Option<u8>,
    pub from_id: Option<GnuId>,
    pub version: Option<i32>,
    pub version_vp: Option<i32>,
    pub version_ex_number: Option<i16>,
    pub version_ex_prefix: Option<Bytes>,
    //
    pub channel_id: Option<GnuId>,
    pub broadcast_group: Option<BroadcastGroup>,
    //
    pub channel_packet: Option<PcpChannel>,
    //
    pub host: Option<PcpHost>,
}

impl PcpBroadcast {
    pub fn parse(atom: &Atom) -> Result<PcpBroadcast, AtomParseError> {
        if !(atom.id() == Id4::PCP_BCST && atom.is_parent()) {
            return Err(AtomParseError::NotFoundValue);
        }

        let mut p = PcpBroadcast::default();
        for a in atom.as_parent().childs() {
            if a.is_child() {
                let a = a.as_child();
                match a.id() {
                    Id4::PCP_BCST_TTL => p.ttl = Some(decode_u8(a)?),
                    Id4::PCP_BCST_HOPS => p.hops = Some(decode_u8(a)?),
                    Id4::PCP_BCST_FROM => p.from_id = Some(decode_gnuid(a)?),
                    Id4::PCP_BCST_VERSION => p.version = Some(decode_i32(a)?),
                    Id4::PCP_BCST_VERSION_VP => p.version_vp = Some(decode_i32(a)?),
                    Id4::PCP_BCST_VERSION_EX_NUMBER => p.version_ex_number = Some(decode_i16(a)?),
                    Id4::PCP_BCST_VERSION_EX_PREFIX => p.version_ex_prefix = Some(decode_bytes(a)?), // Bytes
                    Id4::PCP_BCST_CHANID => p.channel_id = Some(decode_gnuid(a)?),
                    Id4::PCP_BCST_GROUP => p.broadcast_group = Some(decode_u8(a)?.into()),
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            } else {
                match a.id() {
                    Id4::PCP_CHAN => p.channel_packet = Some(PcpChannel::parse(a)?),
                    Id4::PCP_HOST => p.host = Some(PcpHost::parse(a)?),
                    _ => {
                        warn!("unkown atom arrived :{:?}", &a)
                    }
                }
            }
        }

        Ok(p)
    }
}

#[cfg(test)]
mod t {
    use super::BroadcastGroup;

    #[test]
    fn test_groups() {
        let g = BroadcastGroup::TO_ALL;
        assert_eq!(g.is_all(), true);
        assert_eq!(g.has_root(), true);
        assert_eq!(g.has_trackers(), true);
        assert_eq!(g.has_relays(), true);

        let g = BroadcastGroup::TO_ROOT;
        assert_eq!(g.is_all(), false);
        assert_eq!(g.has_root(), true);
        assert_eq!(g.has_trackers(), false);
        assert_eq!(g.has_relays(), false);

        let g = BroadcastGroup::TO_RELAYS;
        assert_eq!(g.is_all(), false);
        assert_eq!(g.has_root(), false);
        assert_eq!(g.has_trackers(), false);
        assert_eq!(g.has_relays(), true);
    }
}
